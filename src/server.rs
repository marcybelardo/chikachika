//! Loopback HTTP hosting and bounded browser snapshot delivery.
//!
//! [`OverlayHub`] is the single in-memory publication point shared by the
//! editor and HTTP adapters. It stores the authoritative model and its current
//! browser projection together, while each subscriber receives only the latest
//! value through a bounded Tokio watch channel.

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

#[derive(Clone)]
struct ServerState {
    hub: OverlayHub,
    shutdown: watch::Receiver<bool>,
    // The sender keeps the direct-test router's shutdown receiver open. The
    // production handle owns the sender that actually changes this value.
    _shutdown_signal: watch::Sender<bool>,
    keepalive_interval: Duration,
}

impl ServerState {
    fn new(
        hub: OverlayHub,
        shutdown: watch::Receiver<bool>,
        shutdown_signal: watch::Sender<bool>,
        keepalive_interval: Duration,
    ) -> Self {
        Self {
            hub,
            shutdown,
            _shutdown_signal: shutdown_signal,
            keepalive_interval,
        }
    }
}

/// The loopback address used by the local web server.
pub const DEFAULT_BIND_ADDRESS: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// The stable port used by the normal application run.
pub const DEFAULT_PORT: u16 = 51737;

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

struct OverlayEntry {
    overlay: Overlay,
    sender: watch::Sender<BrowserRepresentation>,
}

/// Synchronous shared state for current overlays and browser subscribers.
///
/// The map lock is held only for map and channel operations. Projection and
/// HTML/JSON rendering happen before or after that lock, and a stream never
/// retains a map guard for its lifetime.
#[derive(Clone, Default)]
pub struct OverlayHub {
    entries: Arc<Mutex<HashMap<OverlayId, OverlayEntry>>>,
}

impl OverlayHub {
    /// Creates an empty hub.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an overlay and seeds its current browser projection.
    pub fn register(&self, overlay: Overlay) -> Result<(), HubError> {
        let id = overlay.id();
        let representation = browser::project(&overlay);
        let mut entries = self.lock_entries()?;
        if entries.contains_key(&id) {
            return Err(HubError::Duplicate { id });
        }

        let (sender, _receiver) = watch::channel(representation);
        entries.insert(id, OverlayEntry { overlay, sender });
        Ok(())
    }

    /// Publishes a replacement with a strictly newer revision.
    ///
    /// Equal revisions are accepted only when the complete model is identical;
    /// equal but different models are conflicts, and older revisions are stale.
    pub fn publish(&self, overlay: &Overlay) -> Result<PublishResult, HubError> {
        let id = overlay.id();
        // Rendering has no map guard. The resulting value is still compared and
        // published while holding the same guard as the model replacement.
        let representation = browser::project(overlay);
        let mut entries = self.lock_entries()?;
        let entry = entries.get_mut(&id).ok_or(HubError::Unknown { id })?;

        match overlay.revision().cmp(&entry.overlay.revision()) {
            std::cmp::Ordering::Greater => {
                entry.overlay = overlay.clone();
                entry.sender.send_replace(representation);
                Ok(PublishResult::Published)
            }
            std::cmp::Ordering::Equal if overlay == &entry.overlay => Ok(PublishResult::Unchanged),
            std::cmp::Ordering::Equal => Err(HubError::Conflict {
                id,
                revision: overlay.revision(),
            }),
            std::cmp::Ordering::Less => Err(HubError::Stale {
                id,
                current_revision: entry.overlay.revision(),
                incoming_revision: overlay.revision(),
            }),
        }
    }

    /// Removes an overlay, closing all streams subscribed to its sender.
    pub fn remove(&self, id: OverlayId) -> Result<Option<Overlay>, HubError> {
        let removed = {
            let mut entries = self.lock_entries()?;
            entries
                .remove(&id)
                .map(|entry| (entry.overlay, entry.sender))
        };

        Ok(removed.map(|(overlay, sender)| {
            // Explicitly drop the sender after the map guard is gone. Receivers
            // observe closure and no lock is held while they finish.
            drop(sender);
            overlay
        }))
    }

    /// Returns a cloned current model snapshot, if the ID is registered.
    pub fn snapshot(&self, id: OverlayId) -> Result<Option<Overlay>, HubError> {
        let entries = self.lock_entries()?;
        Ok(entries.get(&id).map(|entry| entry.overlay.clone()))
    }

    /// Returns a receiver seeded with the current projection, if registered.
    pub fn subscribe(
        &self,
        id: OverlayId,
    ) -> Result<watch::Receiver<BrowserRepresentation>, HubError> {
        let entries = self.lock_entries()?;
        entries
            .get(&id)
            .map(|entry| entry.sender.subscribe())
            .ok_or(HubError::Unknown { id })
    }

    fn lock_entries(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<OverlayId, OverlayEntry>>, HubError> {
        self.entries.lock().map_err(|_| HubError::LockPoisoned)
    }
}

/// Outcome of a revision-checked publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishResult {
    Published,
    Unchanged,
}

/// Errors from synchronous hub operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HubError {
    Duplicate {
        id: OverlayId,
    },
    Unknown {
        id: OverlayId,
    },
    Stale {
        id: OverlayId,
        current_revision: u64,
        incoming_revision: u64,
    },
    Conflict {
        id: OverlayId,
        revision: u64,
    },
    LockPoisoned,
}

impl fmt::Display for HubError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate { id } => write!(formatter, "overlay {id} is already registered"),
            Self::Unknown { id } => write!(formatter, "overlay {id} is not registered"),
            Self::Stale {
                id,
                current_revision,
                incoming_revision,
            } => write!(
                formatter,
                "overlay {id} revision {incoming_revision} is stale; current revision is {current_revision}"
            ),
            Self::Conflict { id, revision } => {
                write!(
                    formatter,
                    "overlay {id} has a conflicting revision {revision}"
                )
            }
            Self::LockPoisoned => formatter.write_str("overlay hub lock is poisoned"),
        }
    }
}

impl Error for HubError {}

/// Builds a local web-server router backed by a fresh empty hub.
pub fn router() -> Router {
    let (shutdown_signal, shutdown) = watch::channel(false);
    router_with_state(ServerState::new(
        OverlayHub::new(),
        shutdown,
        shutdown_signal,
        KEEPALIVE_INTERVAL,
    ))
}

/// Builds a local web-server router backed by `hub`.
pub fn router_with_hub(hub: OverlayHub) -> Router {
    let (shutdown_signal, shutdown) = watch::channel(false);
    router_with_state(ServerState::new(
        hub,
        shutdown,
        shutdown_signal,
        KEEPALIVE_INTERVAL,
    ))
}

fn router_with_state(state: ServerState) -> Router {
    Router::new()
        .route("/ping", get(ping))
        .route("/overlay/:id", get(render_overlay))
        .route("/overlay/:id/events", get(overlay_events))
        .with_state(state)
}

/// Starts the server on the stable default address, `127.0.0.1:51737`.
pub fn start() -> Result<ServerHandle, ServerError> {
    start_with_hub(OverlayHub::new())
}

/// Starts the server on the stable default address with shared application state.
pub fn start_with_hub(hub: OverlayHub) -> Result<ServerHandle, ServerError> {
    start_on_port_with_hub(DEFAULT_PORT, hub)
}

/// Starts the server on a loopback port in a dedicated thread.
///
/// Port `0` is useful for tests and asks the operating system to select an
/// available ephemeral port. Normal application startup should use [`start`]
/// so copied browser-source URLs remain stable.
pub fn start_on_port(port: u16) -> Result<ServerHandle, ServerError> {
    start_on_port_with_hub(port, OverlayHub::new())
}

/// Starts the server on a loopback port with the supplied shared hub.
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
                    Ok(local_addr) => local_addr,
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

/// A running loopback web server.
///
/// Call [`ServerHandle::shutdown`] during application shutdown to signal the
/// server and join its dedicated thread. Dropping the handle also signals the
/// server, but cannot synchronously join its thread.
pub struct ServerHandle {
    address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    stream_shutdown: Option<watch::Sender<bool>>,
    thread: Option<JoinHandle<Result<(), ServerError>>>,
}

impl ServerHandle {
    /// Returns the address successfully bound by this server.
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    /// Gracefully stops the server and joins its dedicated thread.
    pub fn shutdown(mut self) -> Result<(), ServerError> {
        if let Some(stream_shutdown) = self.stream_shutdown.take() {
            let _ = stream_shutdown.send(true);
        }
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }

        match self.thread.take() {
            Some(thread) => match thread.join() {
                Ok(result) => result,
                Err(_) => Err(ServerError::ThreadPanicked),
            },
            None => Ok(()),
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(stream_shutdown) = self.stream_shutdown.take() {
            let _ = stream_shutdown.send(true);
        }
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

async fn ping() -> &'static str {
    "pong"
}

fn parse_id(id: &str) -> Option<OverlayId> {
    uuid::Uuid::parse_str(id).ok().map(OverlayId::from_uuid)
}

async fn render_overlay(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    let Some(id) = parse_id(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(Some(overlay)) = state.hub.snapshot(id) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let body = browser::render(&overlay);
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
    let shutdown_signal = state._shutdown_signal.clone();
    let keepalive_interval = state.keepalive_interval;

    let output = stream! {
        // Keep the direct-test router's sender alive for the stream lifetime;
        // production additionally retains the sender in ServerHandle.
        let _shutdown_signal = shutdown_signal;
        // subscribe() and this borrow_and_update() are intentionally adjacent:
        // the receiver is registered before the first snapshot is read, so a
        // publication racing a request is either the first value or a later
        // changed() value, never an unobserved update.
        let mut keepalive = Box::pin(tokio::time::sleep(keepalive_interval));
        let first = receiver.borrow_and_update().clone();
        yield Ok::<Event, Infallible>(snapshot_event(&first));

        loop {
            tokio::select! {
                shutdown_changed = shutdown.changed() => {
                    if shutdown_changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                changed = receiver.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    let snapshot = receiver.borrow_and_update().clone();
                    yield Ok(snapshot_event(&snapshot));
                }
                _ = &mut keepalive => {
                    yield Ok(Event::default().comment("keepalive"));
                    keepalive.as_mut().reset(tokio::time::Instant::now() + KEEPALIVE_INTERVAL);
                }
            }
        }
    };

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

/// Errors reported while starting or stopping the local web server.
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind(error) => write!(formatter, "could not bind the web server: {error}"),
            Self::Runtime(error) => {
                write!(formatter, "could not create the Tokio runtime: {error}")
            }
            Self::Thread(error) => {
                write!(formatter, "could not start the web-server thread: {error}")
            }
            Self::Serve(error) => {
                write!(formatter, "the web server stopped with an error: {error}")
            }
            Self::ThreadPanicked => formatter.write_str("the web-server thread panicked"),
            Self::StartupChannelClosed => {
                formatter.write_str("the web-server thread exited before reporting its address")
            }
        }
    }
}

impl Error for ServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bind(error) | Self::Runtime(error) | Self::Thread(error) | Self::Serve(error) => {
                Some(error)
            }
            Self::ThreadPanicked | Self::StartupChannelClosed => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, header};
    use http_body_util::BodyExt;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::{Arc, Barrier, mpsc};
    use std::time::Duration;
    use tower::ServiceExt;

    use crate::browser;
    use crate::model::Overlay;

    fn overlay() -> Overlay {
        Overlay::with_dimensions("Starting Soon", 1280, 720).expect("valid test overlay")
    }

    fn changed_overlay(base: &Overlay, name: &str) -> Overlay {
        let mut changed = base.clone();
        changed.rename(name).expect("valid test name");
        changed
    }

    fn wire_id(overlay: &Overlay) -> String {
        overlay.id().to_string()
    }

    async fn response_for(router: Router, path: &str) -> Response {
        router
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("valid test request"),
            )
            .await
            .expect("router response")
    }

    async fn next_frame(body: &mut Body) -> Option<Vec<u8>> {
        loop {
            let frame = tokio::time::timeout(Duration::from_secs(2), body.frame())
                .await
                .expect("body frame did not arrive within the test bound")?;
            let frame = frame.expect("body frame error");
            if let Ok(data) = frame.into_data() {
                return Some(data.to_vec());
            }
        }
    }

    fn event_snapshot(frame: &[u8]) -> serde_json::Value {
        let text = std::str::from_utf8(frame).expect("SSE frame is UTF-8");
        assert!(
            text.starts_with("event: snapshot\n"),
            "unexpected SSE frame: {text:?}"
        );
        assert!(!text.contains("\nid:"));
        assert!(!text.contains("\nretry:"));
        let data = text
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .expect("snapshot event data");
        serde_json::from_str(data).expect("snapshot event JSON")
    }

    #[test]
    fn register_seeds_snapshot_and_rejects_duplicate_id() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = initial.id();

        hub.register(initial.clone()).expect("register overlay");
        assert_eq!(
            hub.snapshot(id).expect("snapshot lock"),
            Some(initial.clone())
        );
        assert!(matches!(
            hub.register(initial),
            Err(HubError::Duplicate { id: duplicate }) if duplicate == id
        ));
    }

    #[test]
    fn unknown_lookup_and_publish_are_explicit() {
        let hub = OverlayHub::new();
        let unknown = overlay();
        let id = unknown.id();

        assert_eq!(hub.snapshot(id).expect("snapshot lock"), None);
        assert!(matches!(hub.subscribe(id), Err(HubError::Unknown { id: found }) if found == id));
        assert!(
            matches!(hub.publish(&unknown), Err(HubError::Unknown { id: found }) if found == id)
        );
        assert_eq!(hub.remove(id).expect("remove lock"), None);
    }

    #[test]
    fn publish_revision_outcomes_replace_and_keep_zero_subscriber_state_current() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = initial.id();
        hub.register(initial.clone()).expect("register overlay");

        let newer = changed_overlay(&initial, "Live");
        assert_eq!(hub.publish(&newer), Ok(PublishResult::Published));
        assert_eq!(
            hub.snapshot(id).expect("snapshot lock"),
            Some(newer.clone())
        );
        assert_eq!(hub.publish(&newer), Ok(PublishResult::Unchanged));

        let stale = initial.clone();
        assert!(matches!(
            hub.publish(&stale),
            Err(HubError::Stale {
                id: found,
                current_revision: 1,
                incoming_revision: 0
            }) if found == id
        ));

        let conflicting = changed_overlay(&initial, "Another Live");
        assert!(matches!(
            hub.publish(&conflicting),
            Err(HubError::Conflict { id: found, revision: 1 }) if found == id
        ));

        let receiver = hub.subscribe(id).expect("subscribe current state");
        assert_eq!(receiver.borrow().revision(), 1);
    }

    #[tokio::test]
    async fn multiple_subscribers_converge_on_latest_snapshot() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = initial.id();
        hub.register(initial.clone()).expect("register overlay");
        let mut first = hub.subscribe(id).expect("first subscriber");
        let mut second = hub.subscribe(id).expect("second subscriber");

        let latest = changed_overlay(&initial, "Latest");
        hub.publish(&latest).expect("publish latest");
        first.changed().await.expect("first update");
        second.changed().await.expect("second update");
        assert_eq!(first.borrow_and_update().revision(), 1);
        assert_eq!(second.borrow_and_update().revision(), 1);
    }

    #[test]
    fn subscribe_and_publish_race_has_no_lost_update() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = initial.id();
        hub.register(initial.clone()).expect("register overlay");
        let latest = changed_overlay(&initial, "Published");
        let barrier = Arc::new(Barrier::new(2));
        let thread_hub = hub.clone();
        let thread_barrier = barrier.clone();
        let subscriber = std::thread::spawn(move || {
            thread_barrier.wait();
            thread_hub
                .subscribe(id)
                .expect("subscribe race participant")
        });
        barrier.wait();
        hub.publish(&latest).expect("publish race participant");
        let mut receiver = subscriber.join().expect("subscriber thread");

        let first_revision = receiver.borrow_and_update().revision();
        assert!(first_revision <= 1);
        if first_revision == 0 {
            assert!(receiver.has_changed().expect("receiver remains open"));
            assert_eq!(receiver.borrow_and_update().revision(), 1);
        }
    }

    #[test]
    fn publish_and_remove_race_leaves_removed_id_unknown() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = initial.id();
        hub.register(initial.clone()).expect("register overlay");
        let latest = changed_overlay(&initial, "Published");
        let barrier = Arc::new(Barrier::new(2));
        let thread_hub = hub.clone();
        let thread_barrier = barrier.clone();
        let publisher = std::thread::spawn(move || {
            thread_barrier.wait();
            thread_hub.publish(&latest)
        });
        barrier.wait();
        let removed = hub.remove(id).expect("remove race participant");
        assert!(removed.is_some());
        let result = publisher.join().expect("publisher thread");
        assert!(matches!(
            result,
            Ok(PublishResult::Published) | Err(HubError::Unknown { .. })
        ));
        assert_eq!(hub.snapshot(id).expect("snapshot after removal"), None);
    }

    #[test]
    fn removal_closes_existing_receiver() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = initial.id();
        hub.register(initial).expect("register overlay");
        let receiver = hub.subscribe(id).expect("subscribe overlay");
        assert!(hub.remove(id).expect("remove overlay").is_some());
        assert!(receiver.has_changed().is_err());
    }

    #[tokio::test]
    async fn router_preserves_ping_and_renders_exact_html_headers() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = wire_id(&initial);
        hub.register(initial.clone()).expect("register overlay");

        let ping = response_for(router_with_hub(hub.clone()), "/ping").await;
        assert_eq!(ping.status(), StatusCode::OK);
        assert_eq!(ping.into_body().collect().await.unwrap().to_bytes(), "pong");

        let response = response_for(router_with_hub(hub), &format!("/overlay/{id}")).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-cache"
        );
        assert_eq!(response.headers().get(header::PRAGMA).unwrap(), "no-cache");
        assert_eq!(response.headers().get(header::EXPIRES).unwrap(), "0");
        let html = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(html, browser::render(&initial));
    }

    #[tokio::test]
    async fn malformed_unknown_and_removed_routes_return_404_before_streaming() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = wire_id(&initial);
        hub.register(initial).expect("register overlay");

        for path in [
            "/overlay/not-an-id",
            "/overlay/00000000-0000-0000-0000-000000000000",
            "/overlay/not-an-id/events",
        ] {
            let response = response_for(router_with_hub(hub.clone()), path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "path {path}");
        }
        hub.remove(OverlayId::from_uuid(
            uuid::Uuid::parse_str(&id).expect("test ID"),
        ))
        .expect("remove overlay");
        let response = response_for(router_with_hub(hub), &format!("/overlay/{id}/events")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn events_send_current_then_next_complete_named_snapshot() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = wire_id(&initial);
        hub.register(initial.clone()).expect("register overlay");
        let response = response_for(
            router_with_hub(hub.clone()),
            &format!("/overlay/{id}/events"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/event-stream"
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-cache"
        );
        let mut body = response.into_body();
        let first = next_frame(&mut body).await.expect("first snapshot frame");
        let first_json = event_snapshot(&first);
        assert_eq!(first_json["overlay_id"], id);
        assert_eq!(first_json["revision"], 0);
        assert_eq!(first_json["canvas"]["width"], 1280);
        assert_eq!(first_json["canvas"]["height"], 720);
        assert!(first_json["text_widget"].is_null());

        let latest = changed_overlay(&initial, "Updated");
        hub.publish(&latest).expect("publish update");
        let next = next_frame(&mut body).await.expect("next snapshot frame");
        let next_json = event_snapshot(&next);
        assert_eq!(next_json["revision"], 1);
        assert_eq!(next_json["overlay_id"], id);
    }

    #[tokio::test]
    async fn reconnect_starts_at_current_latest_without_replay() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = wire_id(&initial);
        hub.register(initial.clone()).expect("register overlay");
        let latest = changed_overlay(&initial, "Latest");
        hub.publish(&latest).expect("publish latest");

        let response = response_for(router_with_hub(hub), &format!("/overlay/{id}/events")).await;
        let mut body = response.into_body();
        let first = next_frame(&mut body).await.expect("reconnect snapshot");
        assert_eq!(event_snapshot(&first)["revision"], 1);
    }

    #[tokio::test]
    async fn rapid_updates_are_bounded_to_latest_value() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = wire_id(&initial);
        hub.register(initial.clone()).expect("register overlay");
        let response = response_for(
            router_with_hub(hub.clone()),
            &format!("/overlay/{id}/events"),
        )
        .await;
        let mut body = response.into_body();
        next_frame(&mut body).await.expect("initial snapshot");

        let mut latest = initial;
        for revision in 1..=32 {
            latest = changed_overlay(&latest, &format!("Revision {revision}"));
            hub.publish(&latest).expect("rapid publication");
        }
        let frame = next_frame(&mut body)
            .await
            .expect("bounded latest snapshot");
        assert_eq!(event_snapshot(&frame)["revision"], 32);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), next_frame(&mut body))
                .await
                .is_err(),
            "stream unexpectedly queued an unbounded history"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn keepalive_is_emitted_under_paused_tokio_time() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = wire_id(&initial);
        hub.register(initial).expect("register overlay");
        let response = response_for(
            router_with_hub(hub.clone()),
            &format!("/overlay/{id}/events"),
        )
        .await;
        let mut body = response.into_body();
        next_frame(&mut body).await.expect("initial snapshot");

        tokio::time::advance(KEEPALIVE_INTERVAL).await;
        tokio::task::yield_now().await;
        let keepalive = next_frame(&mut body).await.expect("keepalive frame");
        assert_eq!(keepalive, b": keepalive\n\n");
    }

    #[tokio::test]
    async fn stream_harness_reads_bounded_frames_without_hanging() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = wire_id(&initial);
        hub.register(initial.clone()).expect("register overlay");
        let response = response_for(
            router_with_hub(hub.clone()),
            &format!("/overlay/{id}/events"),
        )
        .await;
        let mut body = response.into_body();
        let first = next_frame(&mut body).await.expect("bounded first frame");
        assert_eq!(event_snapshot(&first)["revision"], 0);
        let latest = changed_overlay(&initial, "Bounded");
        hub.publish(&latest).expect("bounded update");
        let next = next_frame(&mut body).await.expect("bounded next frame");
        assert_eq!(event_snapshot(&next)["revision"], 1);
    }

    #[test]
    fn shutdown_closes_active_sse_stream_and_releases_port() {
        let hub = OverlayHub::new();
        let initial = overlay();
        let id = wire_id(&initial);
        hub.register(initial).expect("register overlay");
        let server = start_on_port_with_hub(0, hub).expect("start test server");
        let address = server.local_addr();
        let mut stream = TcpStream::connect(address).expect("connect to SSE route");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set SSE read timeout");
        write!(
            stream,
            "GET /overlay/{id}/events HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .expect("send SSE request");

        let mut response = Vec::new();
        while !response.windows(4).any(|window| window == b"\r\n\r\n") {
            let mut buffer = [0_u8; 256];
            let read = stream.read(&mut buffer).expect("read SSE headers");
            assert_ne!(read, 0, "SSE server closed before sending headers");
            response.extend_from_slice(&buffer[..read]);
        }
        assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
        // The 200 response headers prove the long-lived SSE request is active;
        // initial event framing is covered by the direct-router tests above.
        let (shutdown_sender, shutdown_receiver) = mpsc::channel();
        std::thread::spawn(move || {
            shutdown_sender
                .send(server.shutdown())
                .expect("report server shutdown result");
        });
        let result = shutdown_receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("server shutdown did not finish with active SSE stream")
            .expect("server shuts down with active SSE stream");
        assert_eq!(result, ());

        let rebound = start_on_port(address.port()).expect("released port can be rebound");
        rebound.shutdown().expect("stop rebound server");
    }

    #[test]
    fn test_server_uses_ephemeral_loopback_port() {
        let server = start_on_port(0).expect("start test server");
        let address = server.local_addr();
        let shutdown = server.shutdown();

        assert_eq!(address.ip(), DEFAULT_BIND_ADDRESS);
        assert_ne!(address.port(), 0);
        shutdown.expect("stop test server");
    }

    #[test]
    fn configured_port_conflict_is_visible_and_non_destructive() {
        let server = start_on_port(0).expect("start test server");
        let address = server.local_addr();

        match start_on_port(address.port()) {
            Err(ServerError::Bind(error)) => {
                assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
            }
            Err(error) => panic!("expected bind conflict, got {error}"),
            Ok(conflicting_server) => {
                let _ = conflicting_server.shutdown();
                panic!("expected second server startup to fail");
            }
        }

        let shutdown = server.shutdown();
        shutdown.expect("stop test server");
    }

    #[test]
    fn shutdown_releases_configured_port() {
        let server = start_on_port(0).expect("start test server");
        let port = server.local_addr().port();
        server.shutdown().expect("stop test server");

        let restarted_server = start_on_port(port).expect("restart test server on released port");
        restarted_server
            .shutdown()
            .expect("stop restarted test server");
    }
}
