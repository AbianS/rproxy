//! Test server utilities for integration tests

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// A mock upstream server for testing
#[allow(dead_code)]
pub struct MockUpstream {
    pub addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

#[allow(dead_code)]
impl MockUpstream {
    /// Start a mock upstream server that responds with a simple message
    pub async fn start() -> Self {
        Self::start_with_response(200, "Hello from upstream").await
    }

    /// Start a mock upstream server with a custom response
    pub async fn start_with_response(status: u16, body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            tokio::select! {
                _ = Self::run_server(listener, status, body) => {}
                _ = shutdown_rx => {}
            }
        });

        Self {
            addr,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    async fn run_server(listener: TcpListener, status: u16, body: &'static str) {
        loop {
            if let Ok((mut stream, _)) = listener.accept().await {
                let response = format!(
                    "HTTP/1.1 {} OK\r\n\
                     Content-Type: text/plain\r\n\
                     Content-Length: {}\r\n\
                     \r\n\
                     {}",
                    status,
                    body.len(),
                    body
                );

                // Read the request first
                let mut buf = [0u8; 4096];
                let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;

                // Send response
                let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;
            }
        }
    }

    /// Get the upstream address as a string
    pub fn address(&self) -> String {
        self.addr.to_string()
    }
}

impl Drop for MockUpstream {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// A mock WebSocket upstream for testing
#[allow(dead_code)]
pub struct MockWebSocketUpstream {
    pub addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

#[allow(dead_code)]
impl MockWebSocketUpstream {
    /// Start a mock WebSocket server
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            tokio::select! {
                _ = Self::run_server(listener) => {}
                _ = shutdown_rx => {}
            }
        });

        Self {
            addr,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    async fn run_server(listener: TcpListener) {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                // Accept WebSocket connection
                let ws = tokio_tungstenite::accept_async(stream).await;
                if let Ok(mut ws) = ws {
                    use futures_util::StreamExt;
                    // Echo messages back
                    while let Some(msg) = ws.next().await {
                        if msg.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Get the upstream address as a string
    pub fn address(&self) -> String {
        self.addr.to_string()
    }
}

impl Drop for MockWebSocketUpstream {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
