use std::error::Error;
use std::fmt;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tokio::sync::oneshot;

/// The loopback address used by the local web server.
pub const DEFAULT_BIND_ADDRESS: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// The stable port used by the normal application run.
pub const DEFAULT_PORT: u16 = 51737;

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

/// A running loopback web server.
///
/// Call [`ServerHandle::shutdown`] during application shutdown to signal the
/// server and join its dedicated thread. Dropping the handle also signals the
/// server, but cannot synchronously join its thread.
pub struct ServerHandle {
    address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<Result<(), ServerError>>>,
}

impl ServerHandle {
    /// Returns the address successfully bound by this server.
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    /// Gracefully stops the server and joins its dedicated thread.
    pub fn shutdown(mut self) -> Result<(), ServerError> {
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
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

/// Builds the local web-server router.
///
/// The router is kept separate from startup so it can be reused by callers
/// that own their Tokio runtime or listener.
pub fn router() -> Router {
    Router::new().route("/ping", get(ping))
}

/// Starts the server on the stable default address, `127.0.0.1:51737`.
pub fn start() -> Result<ServerHandle, ServerError> {
    start_on_port(DEFAULT_PORT)
}

/// Starts the server on a loopback port in a dedicated thread.
///
/// Port `0` is useful for tests and asks the operating system to select an
/// available ephemeral port. Normal application startup should use [`start`]
/// so copied browser-source URLs remain stable.
pub fn start_on_port(port: u16) -> Result<ServerHandle, ServerError> {
    let address = SocketAddr::from((DEFAULT_BIND_ADDRESS, port));
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();

    let thread = thread::Builder::new()
        .name("chikachika-web-server".to_owned())
        .spawn(move || {
            let runtime = Builder::new_current_thread()
                .enable_io()
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
                axum::serve(listener, router())
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

async fn ping() -> &'static str {
    "pong"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    fn request(address: SocketAddr, path: &str) -> String {
        let mut stream = TcpStream::connect(address).expect("connect to test server");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set test-client read timeout");
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .expect("send HTTP request");

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .expect("read HTTP response");
        String::from_utf8(response).expect("HTTP response is UTF-8")
    }

    #[test]
    fn ping_returns_pong() {
        let server = start_on_port(0).expect("start test server");
        let response = request(server.local_addr(), "/ping");
        let shutdown = server.shutdown();

        assert!(response.contains("HTTP/1.1 200 OK\r\n"));
        assert_eq!(response.split("\r\n\r\n").nth(1), Some("pong"));
        shutdown.expect("stop test server");
    }

    #[test]
    fn unknown_route_returns_not_found() {
        let server = start_on_port(0).expect("start test server");
        let response = request(server.local_addr(), "/missing");
        let shutdown = server.shutdown();

        assert!(response.contains("HTTP/1.1 404 Not Found\r\n"));
        shutdown.expect("stop test server");
    }
}
