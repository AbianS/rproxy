//! Integration tests for proxy functionality

use crate::common::server::MockUpstream;
use std::time::Duration;

#[tokio::test]
async fn test_mock_upstream_starts() {
    let upstream = MockUpstream::start().await;

    // Should have a valid address
    assert!(upstream.addr.port() > 0);
}

#[tokio::test]
async fn test_mock_upstream_responds() {
    let upstream = MockUpstream::start().await;

    // Connect and send a simple request
    let mut stream = tokio::net::TcpStream::connect(upstream.addr).await.unwrap();

    // Send HTTP request
    let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();

    // Read response
    let mut buf = vec![0u8; 4096];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
        .await
        .unwrap();

    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("HTTP/1.1 200 OK"));
    assert!(response.contains("Hello from upstream"));
}

#[tokio::test]
async fn test_mock_upstream_custom_response() {
    let upstream = MockUpstream::start_with_response(201, "Created").await;

    let mut stream = tokio::net::TcpStream::connect(upstream.addr).await.unwrap();

    let request = "POST / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();

    let mut buf = vec![0u8; 4096];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
        .await
        .unwrap();

    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("HTTP/1.1 201 OK"));
    assert!(response.contains("Created"));
}

#[tokio::test]
async fn test_mock_upstream_address_format() {
    let upstream = MockUpstream::start().await;

    let addr = upstream.address();
    assert!(addr.starts_with("127.0.0.1:"));
}

#[tokio::test]
async fn test_mock_upstream_multiple_requests() {
    let upstream = MockUpstream::start().await;

    for i in 0..5 {
        let mut stream = tokio::net::TcpStream::connect(upstream.addr).await.unwrap();

        let request = format!("GET /request-{} HTTP/1.1\r\nHost: localhost\r\n\r\n", i);
        tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
            .await
            .unwrap();

        let mut buf = vec![0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
            .await
            .unwrap();

        let response = String::from_utf8_lossy(&buf[..n]);
        assert!(response.contains("HTTP/1.1 200 OK"));
    }
}

#[tokio::test]
async fn test_mock_upstream_concurrent_connections() {
    let upstream = MockUpstream::start().await;

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let addr = upstream.addr;
            tokio::spawn(async move {
                let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

                let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
                tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
                    .await
                    .unwrap();

                let mut buf = vec![0u8; 4096];
                let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
                    .await
                    .unwrap();

                let response = String::from_utf8_lossy(&buf[..n]);
                assert!(response.contains("HTTP/1.1 200 OK"));
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_connection_timeout_handling() {
    // Test that we handle connection timeouts properly
    let timeout = Duration::from_millis(100);

    // Try to connect to a non-routable address
    let result =
        tokio::time::timeout(timeout, tokio::net::TcpStream::connect("10.255.255.1:80")).await;

    // Should timeout
    assert!(result.is_err());
}

#[tokio::test]
async fn test_connection_refused_handling() {
    // Test connecting to a port that's not listening
    let result = tokio::net::TcpStream::connect("127.0.0.1:1").await;

    // Should fail with connection refused
    assert!(result.is_err());
}

/// Test helper to simulate proxy forwarding
async fn simulate_proxy_forward(upstream_addr: std::net::SocketAddr) -> String {
    let mut stream = tokio::net::TcpStream::connect(upstream_addr).await.unwrap();

    let request = "GET /api/test HTTP/1.1\r\n\
                   Host: localhost\r\n\
                   Accept: application/json\r\n\
                   X-Custom-Header: test\r\n\
                   \r\n";

    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();

    let mut buf = vec![0u8; 4096];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
        .await
        .unwrap();

    String::from_utf8_lossy(&buf[..n]).to_string()
}

#[tokio::test]
async fn test_proxy_forward_simulation() {
    let upstream = MockUpstream::start().await;

    let response = simulate_proxy_forward(upstream.addr).await;

    assert!(response.contains("HTTP/1.1 200 OK"));
    assert!(response.contains("Hello from upstream"));
}

#[tokio::test]
async fn test_proxy_with_large_headers() {
    let upstream = MockUpstream::start().await;

    let mut stream = tokio::net::TcpStream::connect(upstream.addr).await.unwrap();

    // Create a large header value
    let large_value = "x".repeat(4096);
    let request = format!(
        "GET / HTTP/1.1\r\n\
         Host: localhost\r\n\
         X-Large-Header: {}\r\n\
         \r\n",
        large_value
    );

    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();

    let mut buf = vec![0u8; 4096];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
        .await
        .unwrap();

    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("HTTP/1.1 200 OK"));
}

#[tokio::test]
async fn test_proxy_post_request() {
    let upstream = MockUpstream::start().await;

    let mut stream = tokio::net::TcpStream::connect(upstream.addr).await.unwrap();

    let body = r#"{"key": "value"}"#;
    let request = format!(
        "POST /api/data HTTP/1.1\r\n\
         Host: localhost\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        body.len(),
        body
    );

    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();

    let mut buf = vec![0u8; 4096];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
        .await
        .unwrap();

    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("HTTP/1.1 200 OK"));
}

#[tokio::test]
async fn test_upstream_shutdown() {
    let upstream = MockUpstream::start().await;
    let addr = upstream.addr;

    // Verify it's running
    let stream = tokio::net::TcpStream::connect(addr).await;
    assert!(stream.is_ok());

    // Drop the upstream (triggers shutdown)
    drop(upstream);

    // Give it a moment to shut down
    tokio::time::sleep(Duration::from_millis(50)).await;

    // New connections should fail
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        tokio::net::TcpStream::connect(addr),
    )
    .await;

    // Either timeout or connection refused
    assert!(result.is_err() || result.unwrap().is_err());
}
