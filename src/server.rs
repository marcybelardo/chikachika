//! Loopback HTTP hosting and bounded browser snapshot delivery.
//!
//! [`OverlayHub`] owns running-session delivery revisions separately from the
//! durable model. It retains a per-ID high-water mark across deletion.

use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use async_stream::stream;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tokio::sync::{oneshot, watch};

use crate::browser::{self, BrowserRepresentation};
use crate::model::{Overlay, OverlayId};

pub const DEFAULT_BIND_ADDRESS: Ipv4Addr = Ipv4Addr::LOCALHOST;
pub const DEFAULT_PORT: u16 = 51737;
pub const MAX_SAFE_REVISION: u64 = 9_007_199_254_740_991;
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Clone)]
struct ServerState {
    hub: OverlayHub,
    shutdown: watch::Receiver<bool>,
    _shutdown_signal: watch::Sender<bool>,
    keepalive_interval: Duration,
}
impl ServerState {
    fn new(
        hub: OverlayHub,
        shutdown: watch::Receiver<bool>,
        signal: watch::Sender<bool>,
        keepalive: Duration,
    ) -> Self {
        Self {
            hub,
            shutdown,
            _shutdown_signal: signal,
            keepalive_interval: keepalive,
        }
    }
}

struct OverlayEntry {
    overlay: Overlay,
    revision: u64,
    sender: watch::Sender<BrowserRepresentation>,
}
#[derive(Clone, Default)]
pub struct OverlayHub {
    entries: Arc<Mutex<HashMap<OverlayId, OverlayEntry>>>,
    high_water: Arc<Mutex<HashMap<OverlayId, u64>>>,
}
impl OverlayHub {
    pub fn new() -> Self {
        Self::default()
    }
    /// Registers a new ID at delivery revision zero. A deleted ID resumes above
    /// its retained high-water mark; allocation is checked before insertion.
    pub fn register(&self, overlay: Overlay) -> Result<(), HubError> {
        let id = overlay.id();
        let mut entries = self.lock_entries()?;
        if entries.contains_key(&id) {
            return Err(HubError::Duplicate { id });
        }
        let mut high_water = self.lock_high_water()?;
        let retained = high_water.get(&id).copied();
        let revision = match retained {
            None => 0,
            Some(current) => {
                next_revision(current).ok_or(HubError::RevisionExhausted { id, current })?
            }
        };
        let representation = browser::project(&overlay, revision);
        let (sender, _) = watch::channel(representation);
        entries.insert(
            id,
            OverlayEntry {
                overlay,
                revision,
                sender,
            },
        );
        high_water.insert(id, revision);
        Ok(())
    }
    /// Allocates the next revision from hub state, ignoring any model revision.
    /// No content, high-water, or watch state changes if allocation is exhausted.
    pub fn publish(&self, overlay: &Overlay) -> Result<PublishResult, HubError> {
        let id = overlay.id();
        let mut entries = self.lock_entries()?;
        let current = entries.get(&id).ok_or(HubError::Unknown { id })?;
        if overlay == &current.overlay {
            return Ok(PublishResult::Unchanged);
        }
        let next = next_revision(current.revision).ok_or(HubError::RevisionExhausted {
            id,
            current: current.revision,
        })?;
        let representation = browser::project(overlay, next);
        let mut high_water = self.lock_high_water()?;
        let entry = entries.get_mut(&id).expect("entry checked above");
        entry.overlay = overlay.clone();
        entry.revision = next;
        entry.sender.send_replace(representation);
        high_water.insert(id, next);
        Ok(PublishResult::Published { revision: next })
    }
    pub fn remove(&self, id: OverlayId) -> Result<Option<Overlay>, HubError> {
        let removed = {
            let mut entries = self.lock_entries()?;
            entries
                .remove(&id)
                .map(|entry| (entry.overlay, entry.sender))
        };
        Ok(removed.map(|(overlay, sender)| {
            drop(sender);
            overlay
        }))
    }
    pub fn snapshot(&self, id: OverlayId) -> Result<Option<Overlay>, HubError> {
        Ok(self
            .lock_entries()?
            .get(&id)
            .map(|entry| entry.overlay.clone()))
    }
    pub fn revision(&self, id: OverlayId) -> Result<Option<u64>, HubError> {
        Ok(self.lock_entries()?.get(&id).map(|entry| entry.revision))
    }
    pub fn subscribe(
        &self,
        id: OverlayId,
    ) -> Result<watch::Receiver<BrowserRepresentation>, HubError> {
        self.lock_entries()?
            .get(&id)
            .map(|entry| entry.sender.subscribe())
            .ok_or(HubError::Unknown { id })
    }
    #[cfg(test)]
    fn set_high_water_for_test(&self, id: OverlayId, revision: u64) {
        self.high_water.lock().unwrap().insert(id, revision);
        if let Some(entry) = self.entries.lock().unwrap().get_mut(&id) {
            entry.revision = revision;
        }
    }
    fn lock_entries(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<OverlayId, OverlayEntry>>, HubError> {
        self.entries.lock().map_err(|_| HubError::LockPoisoned)
    }
    fn lock_high_water(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<OverlayId, u64>>, HubError> {
        self.high_water.lock().map_err(|_| HubError::LockPoisoned)
    }
}
fn next_revision(current: u64) -> Option<u64> {
    current
        .checked_add(1)
        .filter(|next| *next <= MAX_SAFE_REVISION)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishResult {
    Published { revision: u64 },
    Unchanged,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HubError {
    Duplicate { id: OverlayId },
    Unknown { id: OverlayId },
    LockPoisoned,
    RevisionExhausted { id: OverlayId, current: u64 },
}
impl fmt::Display for HubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate { id } => write!(f, "overlay {id} is already registered"),
            Self::Unknown { id } => write!(f, "overlay {id} is not registered"),
            Self::LockPoisoned => write!(f, "overlay hub lock is poisoned"),
            Self::RevisionExhausted { id, current } => {
                write!(f, "overlay {id} delivery revision exhausted at {current}")
            }
        }
    }
}
impl Error for HubError {}

pub fn router() -> Router {
    router_with_hub(OverlayHub::new())
}
pub fn router_with_hub(hub: OverlayHub) -> Router {
    let (signal, receiver) = watch::channel(false);
    router_with_state(ServerState::new(hub, receiver, signal, KEEPALIVE_INTERVAL))
}
fn router_with_state(state: ServerState) -> Router {
    Router::new()
        .route("/ping", get(ping))
        .route("/overlay/:id", get(render_overlay))
        .route("/overlay/:id/events", get(overlay_events))
        .with_state(state)
}
pub fn start() -> Result<ServerHandle, ServerError> {
    start_with_hub(OverlayHub::new())
}
pub fn start_with_hub(hub: OverlayHub) -> Result<ServerHandle, ServerError> {
    start_on_port_with_hub(DEFAULT_PORT, hub)
}
pub fn start_on_port(port: u16) -> Result<ServerHandle, ServerError> {
    start_on_port_with_hub(port, OverlayHub::new())
}
pub fn start_on_port_with_hub(port: u16, hub: OverlayHub) -> Result<ServerHandle, ServerError> {
    let address = SocketAddr::from((DEFAULT_BIND_ADDRESS, port));
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (stream_shutdown_sender, stream_shutdown_receiver) = watch::channel(false);
    let stream_shutdown_handle = stream_shutdown_sender.clone();
    let thread = thread::Builder::new()
        .name("chikachika-web-server".to_owned())
        .spawn(move || {
            let runtime = Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .map_err(ServerError::Runtime)?;
            runtime.block_on(async move {
                let listener = match TcpListener::bind(address).await {
                    Ok(listener) => listener,
                    Err(error) => {
                        let _ = ready_sender.send(Err(ServerError::Bind(error)));
                        return Ok(());
                    }
                };
                let local_addr = match listener.local_addr() {
                    Ok(address) => address,
                    Err(error) => {
                        let _ = ready_sender.send(Err(ServerError::Bind(error)));
                        return Ok(());
                    }
                };
                let _ = ready_sender.send(Ok(local_addr));
                axum::serve(
                    listener,
                    router_with_state(ServerState::new(
                        hub,
                        stream_shutdown_receiver,
                        stream_shutdown_sender.clone(),
                        KEEPALIVE_INTERVAL,
                    )),
                )
                .with_graceful_shutdown(async move {
                    let _ = shutdown_receiver.await;
                })
                .await
                .map_err(ServerError::Serve)
            })
        })
        .map_err(ServerError::Thread)?;
    match ready_receiver.recv() {
        Ok(Ok(local_addr)) => Ok(ServerHandle {
            address: local_addr,
            shutdown: Some(shutdown_sender),
            stream_shutdown: Some(stream_shutdown_handle),
            thread: Some(thread),
        }),
        Ok(Err(error)) => {
            let _ = thread.join();
            Err(error)
        }
        Err(_) => match thread.join() {
            Ok(Err(error)) => Err(error),
            Ok(Ok(())) => Err(ServerError::StartupChannelClosed),
            Err(_) => Err(ServerError::ThreadPanicked),
        },
    }
}

pub struct ServerHandle {
    address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    stream_shutdown: Option<watch::Sender<bool>>,
    thread: Option<JoinHandle<Result<(), ServerError>>>,
}
impl ServerHandle {
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }
    pub fn shutdown(mut self) -> Result<(), ServerError> {
        if let Some(sender) = self.stream_shutdown.take() {
            let _ = sender.send(true);
        }
        if let Some(sender) = self.shutdown.take() {
            let _ = sender.send(());
        }
        match self.thread.take() {
            Some(thread) => thread.join().map_err(|_| ServerError::ThreadPanicked)?,
            None => Ok(()),
        }
    }
}
impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(sender) = self.stream_shutdown.take() {
            let _ = sender.send(true);
        }
        if let Some(sender) = self.shutdown.take() {
            let _ = sender.send(());
        }
    }
}

async fn ping() -> &'static str {
    "pong"
}
fn parse_id(id: &str) -> Option<OverlayId> {
    uuid::Uuid::parse_str(id)
        .ok()
        .filter(|id| !id.is_nil() && id.get_version_num() == 4)
        .map(OverlayId::from_uuid)
}
async fn render_overlay(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    let Some(id) = parse_id(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(Some(overlay)) = state.hub.snapshot(id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let revision = state.hub.revision(id).ok().flatten().unwrap_or(0);
    let body = browser::render(&overlay, revision);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
            (header::PRAGMA, "no-cache"),
            (header::EXPIRES, "0"),
        ],
        body,
    )
        .into_response()
}
async fn overlay_events(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    let Some(id) = parse_id(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(mut receiver) = state.hub.subscribe(id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut shutdown = state.shutdown.clone();
    let signal = state._shutdown_signal.clone();
    let interval = state.keepalive_interval;
    let output = stream! { let _signal = signal; let mut keepalive = Box::pin(tokio::time::sleep(interval)); let first = receiver.borrow_and_update().clone(); yield Ok::<Event, Infallible>(snapshot_event(&first)); loop { tokio::select! { changed = shutdown.changed() => { if changed.is_err() || *shutdown.borrow() { break; } }, changed = receiver.changed() => { if changed.is_err() { break; } let snapshot = receiver.borrow_and_update().clone(); yield Ok(snapshot_event(&snapshot)); }, _ = &mut keepalive => { yield Ok(Event::default().comment("keepalive")); keepalive.as_mut().reset(tokio::time::Instant::now() + interval); } } } };
    let mut response = Sse::new(output).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-cache"),
    );
    response
}
fn snapshot_event(snapshot: &BrowserRepresentation) -> Event {
    Event::default()
        .event("snapshot")
        .data(snapshot_json(snapshot))
}
fn snapshot_json(snapshot: &BrowserRepresentation) -> String {
    serde_json::to_string(snapshot).expect("browser snapshot serializes")
}

#[derive(Debug)]
pub enum ServerError {
    Bind(io::Error),
    Runtime(io::Error),
    Thread(io::Error),
    Serve(io::Error),
    ThreadPanicked,
    StartupChannelClosed,
}
impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind(e) => write!(f, "could not bind the web server: {e}"),
            Self::Runtime(e) => write!(f, "could not create the Tokio runtime: {e}"),
            Self::Thread(e) => write!(f, "could not start the web-server thread: {e}"),
            Self::Serve(e) => write!(f, "the web server stopped with an error: {e}"),
            Self::ThreadPanicked => write!(f, "the web-server thread panicked"),
            Self::StartupChannelClosed => write!(
                f,
                "the web-server thread exited before reporting its address"
            ),
        }
    }
}
impl Error for ServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bind(e) | Self::Runtime(e) | Self::Thread(e) | Self::Serve(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser;
    use crate::model::Overlay;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use std::time::Duration;
    use tower::ServiceExt;

    fn overlay() -> Overlay {
        Overlay::with_dimensions("Starting Soon", 1280, 720).unwrap()
    }
    fn changed(base: &Overlay, name: &str) -> Overlay {
        let mut changed = base.clone();
        changed.rename(name).unwrap();
        changed
    }
    async fn response_for(router: Router, path: &str) -> Response {
        router
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }
    async fn next_frame(body: &mut Body) -> Option<Vec<u8>> {
        loop {
            let frame = tokio::time::timeout(Duration::from_secs(2), body.frame())
                .await
                .ok()??;
            let frame = frame.ok()?;
            if let Ok(data) = frame.into_data() {
                return Some(data.to_vec());
            }
        }
    }
    fn event_snapshot(frame: &[u8]) -> serde_json::Value {
        let text = std::str::from_utf8(frame).unwrap();
        let data = text
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .unwrap();
        serde_json::from_str(data).unwrap()
    }

    #[test]
    fn hub_session_revision_lifecycle() {
        let hub = OverlayHub::new();
        let first = overlay();
        let id = first.id();
        hub.register(first.clone()).unwrap();
        assert_eq!(hub.revision(id).unwrap(), Some(0));
        let newer = changed(&first, "Next");
        assert_eq!(
            hub.publish(&newer),
            Ok(PublishResult::Published { revision: 1 })
        );
        hub.remove(id).unwrap();
        let restored = Overlay::from_parts(
            id,
            "Restored".into(),
            newer.canvas(),
            newer.widgets().to_vec(),
        )
        .unwrap();
        hub.register(restored).unwrap();
        assert_eq!(hub.revision(id).unwrap(), Some(2));
    }
    #[test]
    fn hub_revision_exhaustion_is_atomic() {
        let hub = OverlayHub::new();
        let first = overlay();
        let id = first.id();
        hub.register(first.clone()).unwrap();
        hub.set_high_water_for_test(id, MAX_SAFE_REVISION);
        let receiver = hub.subscribe(id).unwrap();
        let changed = changed(&first, "changed");
        assert!(matches!(
            hub.publish(&changed),
            Err(HubError::RevisionExhausted {
                current: MAX_SAFE_REVISION,
                ..
            })
        ));
        assert_eq!(hub.snapshot(id).unwrap(), Some(first.clone()));
        assert_eq!(hub.revision(id).unwrap(), Some(MAX_SAFE_REVISION));
        assert!(!receiver.has_changed().unwrap());
        hub.remove(id).unwrap();
        hub.set_high_water_for_test(id, MAX_SAFE_REVISION);
        let replacement =
            Overlay::from_parts(id, "replacement".into(), first.canvas(), vec![]).unwrap();
        assert!(matches!(
            hub.register(replacement),
            Err(HubError::RevisionExhausted {
                current: MAX_SAFE_REVISION,
                ..
            })
        ));
        assert!(hub.snapshot(id).unwrap().is_none());
    }
    #[test]
    fn coordinator_noop_does_not_publish() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = initial.id();
        hub.register(initial.clone()).unwrap();
        let receiver = hub.subscribe(id).unwrap();
        let same = initial.clone();
        assert_eq!(hub.publish(&same), Ok(PublishResult::Unchanged));
        assert!(!receiver.has_changed().unwrap());
    }
    #[test]
    fn restart_resets_delivery_not_identity() {
        let first = overlay();
        let id = first.id();
        let hub = OverlayHub::new();
        hub.register(first.clone()).unwrap();
        hub.publish(&changed(&first, "changed")).unwrap();
        let restarted = OverlayHub::new();
        restarted.register(first).unwrap();
        assert_eq!(restarted.revision(id).unwrap(), Some(0));
    }
    #[tokio::test]
    async fn coordinator_multi_widget_live_sse() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = initial.id();
        hub.register(initial.clone()).unwrap();
        let response = response_for(
            router_with_hub(hub.clone()),
            &format!("/overlay/{id}/events"),
        )
        .await;
        let mut body = response.into_body();
        assert_eq!(
            event_snapshot(&next_frame(&mut body).await.unwrap())["revision"],
            0
        );
        let latest = changed(&initial, "Live");
        hub.publish(&latest).unwrap();
        let frame = next_frame(&mut body).await.unwrap();
        assert_eq!(event_snapshot(&frame)["revision"], 1);
    }
    #[tokio::test]
    async fn multi_widget_reconnect_and_bounded_latest() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = initial.id();
        hub.register(initial.clone()).unwrap();
        let response = response_for(
            router_with_hub(hub.clone()),
            &format!("/overlay/{id}/events"),
        )
        .await;
        let mut body = response.into_body();
        next_frame(&mut body).await.unwrap();
        let mut latest = initial;
        for n in 1..=8 {
            latest = changed(&latest, &format!("Revision {n}"));
            hub.publish(&latest).unwrap();
        }
        assert_eq!(
            event_snapshot(&next_frame(&mut body).await.unwrap())["revision"],
            8
        );
        let reconnect = response_for(router_with_hub(hub), &format!("/overlay/{id}/events")).await;
        let mut reconnect_body = reconnect.into_body();
        assert_eq!(
            event_snapshot(&next_frame(&mut reconnect_body).await.unwrap())["revision"],
            8
        );
    }
    #[tokio::test]
    async fn overlay_route_renders_current_explicit_revision() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = initial.id();
        hub.register(initial.clone()).unwrap();
        let response = response_for(router_with_hub(hub), &format!("/overlay/{id}")).await;
        let html = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(html, browser::render(&initial, 0));
    }
}
