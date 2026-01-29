---
name: rust-proxy
description: Rust reverse proxy patterns using hyper 1.x, tower middleware, tokio-rustls, and async connection handling. Use when working on proxy.rs, server.rs, middleware, or WebSocket passthrough.
scope: server
---

# Rust Proxy Patterns

## Overview

This skill covers patterns for building high-performance reverse proxies in Rust using hyper 1.x, tower middleware, and the tokio async runtime. It focuses on connection handling, request forwarding, middleware composition, and WebSocket passthrough.

## Core Patterns

### 1. Hyper 1.x Service Pattern

```rust
use hyper::service::service_fn;
use hyper::{Request, Response};

// Service function pattern for hyper 1.x
let service = service_fn(move |req| {
    let handler = handler.clone();
    async move { handler.handle(req).await }
});

// HTTP/1.1 connection
http1::Builder::new()
    .preserve_header_case(true)
    .serve_connection(io, service)
    .with_upgrades()  // Required for WebSocket
    .await
```

**Why this matters:** hyper 1.x uses a different service model than 0.14. The `service_fn` wrapper creates a service from an async function.

### 2. HTTP/2 Auto-Detection

```rust
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use hyper_util::rt::TokioExecutor;

// Auto-detect HTTP/1.1 or HTTP/2 over TLS
AutoBuilder::new(TokioExecutor::new())
    .serve_connection_with_upgrades(io, service)
    .await
```

**Why this matters:** HTTP/2 requires TLS in practice. Using `auto::Builder` handles protocol negotiation automatically via ALPN.

### 3. TLS with ACME (Let's Encrypt)

```rust
use tokio_rustls_acme::{AcmeConfig, caches::DirCache};

let mut acme_config = AcmeConfig::new(&domains)
    .cache(DirCache::new(cache_dir))
    .directory_lets_encrypt(true);

if !email.is_empty() {
    acme_config = acme_config.contact_push(format!("mailto:{}", email));
}

// Creates a stream of TLS-wrapped connections
let mut tls_incoming = acme_config.incoming(tcp_incoming, Vec::new());

while let Some(tls_result) = tls_incoming.next().await {
    let tls_stream = tls_result?;
    // Handle connection...
}
```

**Why this matters:** `tokio-rustls-acme` handles certificate issuance and renewal automatically. The `incoming()` method returns ready-to-use TLS streams.

### 4. Request Forwarding with Timeout

```rust
use tokio::time::timeout;
use hyper::client::conn::http1::Builder as Http1Builder;

// Connect with timeout
let stream = timeout(
    timeout_duration,
    TcpStream::connect(upstream_addr)
).await
    .map_err(|_| ProxyError::UpstreamTimeout)?
    .map_err(|e| ProxyError::UpstreamConnection(e.to_string()))?;

let io = TokioIo::new(stream);

// HTTP/1.1 client connection
let (mut sender, conn) = Http1Builder::new()
    .preserve_header_case(true)
    .handshake(io)
    .await?;

// Spawn connection handler
tokio::spawn(async move {
    if let Err(e) = conn.await {
        debug!("Upstream connection closed: {}", e);
    }
});

// Forward request
let response = sender.send_request(req).await?;
```

**Why this matters:** Timeout prevents hanging on unresponsive upstreams. Spawning the connection handler allows the connection to be reused.

### 5. Forwarding Headers

```rust
fn add_forwarding_headers(req: &mut Request<Incoming>, client_ip: IpAddr, is_tls: bool) {
    let headers = req.headers_mut();

    // X-Forwarded-For (append to existing)
    let xff = if let Some(existing) = headers.get("x-forwarded-for") {
        format!("{}, {}", existing.to_str().unwrap_or(""), client_ip)
    } else {
        client_ip.to_string()
    };
    headers.insert("x-forwarded-for", xff.parse().unwrap());

    // X-Real-IP (only if not present)
    if !headers.contains_key("x-real-ip") {
        headers.insert("x-real-ip", client_ip.to_string().parse().unwrap());
    }

    // X-Forwarded-Proto
    let proto = if is_tls { "https" } else { "http" };
    headers.insert("x-forwarded-proto", proto.parse().unwrap());
}
```

**Why this matters:** Backend applications need to know the original client IP and protocol for logging, security, and redirect generation.

### 6. Hop-by-Hop Header Removal

```rust
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
];

fn strip_hop_by_hop_headers(req: &mut Request<Incoming>) {
    let headers = req.headers_mut();
    for header in HOP_BY_HOP {
        headers.remove(*header);
    }
}
```

**Why this matters:** Hop-by-hop headers are connection-specific and must not be forwarded to the upstream.

### 7. WebSocket Detection

```rust
pub fn is_websocket_upgrade(req: &Request<Incoming>) -> bool {
    let get_header = |key: &str| {
        req.headers()
            .get(key)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_lowercase())
    };

    get_header("connection")
        .map(|v| v.contains("upgrade"))
        .unwrap_or(false)
        && get_header("upgrade")
            .map(|v| v.contains("websocket"))
            .unwrap_or(false)
}
```

**Why this matters:** WebSocket upgrades need special handling - the connection must be passed through bidirectionally.

### 8. Middleware Stack Pattern

```rust
pub struct MiddlewareStack {
    rate_limiter: RateLimitMiddleware,
    filter: FilterMiddleware,
    cache: CacheMiddleware,
    security: SecurityMiddleware,
}

impl MiddlewareStack {
    pub async fn handle(
        &self,
        req: Request<Incoming>,
        remote_addr: SocketAddr,
        proxy: &ProxyHandler,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        // 1. Rate limiting
        if !self.rate_limiter.check(remote_addr.ip()) {
            return Ok(error_response(StatusCode::TOO_MANY_REQUESTS));
        }

        // 2. Path filtering
        if self.filter.is_blocked(req.uri().path()) {
            return Ok(error_response(StatusCode::NOT_FOUND));
        }

        // 3. Cache check
        if let Some(cached) = self.cache.get(req.uri().path()).await {
            return Ok(cached);
        }

        // 4. Forward to upstream
        let response = proxy.handle(req).await?;

        // 5. Add security headers
        self.security.add_headers(response.headers_mut());

        Ok(response)
    }
}
```

**Why this matters:** Clear middleware ordering ensures consistent behavior. Rate limiting should be first to protect resources.

## Key Files

| File | Purpose |
|------|---------|
| `server.rs` | HTTP server, TLS, connection handling |
| `proxy.rs` | Request forwarding, headers |
| `middleware/mod.rs` | Middleware stack composition |
| `middleware/rate_limit.rs` | Token bucket rate limiting |
| `middleware/cache.rs` | Response caching |

## Common Pitfalls

1. **Missing `.with_upgrades()`**: WebSocket upgrades silently fail without this on HTTP/1.1 connections.

2. **Not spawning connection handler**: The hyper client connection must be spawned to handle keep-alive properly.

3. **Forwarding hop-by-hop headers**: Causes protocol errors with HTTP/2 upstreams.

4. **Blocking in async context**: Use `tokio::time::timeout` instead of `std::time` sleep.

5. **TLS without ALPN**: HTTP/2 requires ALPN negotiation; use `tokio-rustls-acme` or configure rustls manually.

## Performance Tips

- Use `Arc<T>` for shared state across connections
- Clone handlers before spawning tasks (move semantics)
- Use `bytes::Bytes` for zero-copy body handling
- Prefer streaming responses over buffering
- Use `moka` with TTL for caching (automatic cleanup)
