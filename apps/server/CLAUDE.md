# rproxy Server - Technical Context

> **Context Note**: This is the **server-specific context** for rproxy.
> - Root context: `/CLAUDE.md`

## Overview

This is the Rust reverse proxy server for rproxy, a minimal high-performance proxy for self-hosted web applications. This document contains all technical details for implementation and maintenance.

## Implementation Status

**Completed (MVP):**
- HTTP/1.1 server with hyper 1.x
- HTTP/2 auto-detection over TLS
- TLS with Let's Encrypt (ACME)
- HTTP → HTTPS redirect
- Rate limiting (token bucket per IP)
- Path filtering (block sensitive files)
- Security headers injection
- WebSocket passthrough
- In-memory caching structure

**Pending (Future):**
- Response body caching (requires buffering)
- Compression middleware integration
- Health check endpoints
- Metrics/observability
- Multi-upstream load balancing

---

## Architecture

### Request Flow

```
┌──────────────┐
│   Client     │
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│                    TLS Layer (rustls)                    │
│              tokio-rustls-acme (Let's Encrypt)           │
└──────────────────────────┬───────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────┐
│                   MiddlewareStack                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│  │ Rate Limit  │→ │ Path Filter │→ │ Cache Check │       │
│  │ (governor)  │  │  (globset)  │  │   (moka)    │       │
│  └─────────────┘  └─────────────┘  └─────────────┘       │
└──────────────────────────┬───────────────────────────────┘
                           │
              ┌────────────┴────────────┐
              │                         │
              ▼                         ▼
       ┌─────────────┐          ┌─────────────┐
       │  WebSocket  │          │   Proxy     │
       │ Passthrough │          │   Handler   │
       └─────────────┘          └──────┬──────┘
                                       │
                                       ▼
                                ┌─────────────┐
                                │  Upstream   │
                                │  (Backend)  │
                                └─────────────┘
```

### Module Responsibilities

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point, logging setup |
| `config.rs` | ENV var parsing, defaults |
| `error.rs` | Custom error types |
| `server.rs` | HTTP server, TLS, connection handling |
| `proxy.rs` | Request forwarding, headers |
| `websocket.rs` | WS upgrade detection, passthrough |
| `middleware/` | Middleware stack implementation |

---

## Server (`server.rs`)

### TLS Mode

Uses `tokio-rustls-acme` for automatic Let's Encrypt certificates:

```rust
let mut acme_config = AcmeConfig::new(&config.tls.domains)
    .cache(DirCache::new(cache_dir))
    .directory_lets_encrypt(true);

let mut tls_incoming = acme_config.incoming(tcp_incoming, Vec::new());
```

**HTTP/2 Auto-Detection:**

```rust
use hyper_util::server::conn::auto::Builder as AutoBuilder;

AutoBuilder::new(TokioExecutor::new())
    .serve_connection_with_upgrades(io, service)
```

### Non-TLS Mode

HTTP/1.1 only (HTTP/2 requires TLS in practice):

```rust
http1::Builder::new()
    .preserve_header_case(true)
    .serve_connection(io, service)
    .with_upgrades()
```

### HTTP Redirect

Separate server on port 80 redirects to HTTPS:

```rust
// 301 redirect to https://{host}{path}
Response::builder()
    .status(301)
    .header("Location", redirect_url)
```

---

## Proxy Handler (`proxy.rs`)

### Request Forwarding

1. Add forwarding headers (X-Forwarded-For, X-Real-IP, X-Forwarded-Proto)
2. Strip hop-by-hop headers (Connection, Keep-Alive, etc.)
3. Connect to upstream with timeout
4. Forward request and stream response

```rust
// Timeout-wrapped connection
let stream = tokio::time::timeout(
    timeout_duration,
    TcpStream::connect(upstream_addr)
).await??;
```

### Forwarding Headers

| Header | Value |
|--------|-------|
| `X-Forwarded-For` | Client IP (appended) |
| `X-Real-IP` | Client IP |
| `X-Forwarded-Proto` | `https` or `http` |
| `X-Forwarded-Host` | Original Host header |

---

## Middleware Stack (`middleware/mod.rs`)

Execution order:

1. **Rate Limiting** - Check token bucket, return 429 if exceeded
2. **Path Filtering** - Block sensitive paths, return 404
3. **WebSocket Detection** - Handle WS upgrades separately
4. **Cache Check** - Return cached response if available
5. **Proxy Forward** - Forward to upstream
6. **Security Headers** - Add HSTS, X-Frame-Options, etc.
7. **Cache Store** - Cache response if cacheable (MVP: disabled)

### Rate Limiting (`rate_limit.rs`)

Uses `governor` crate with token bucket algorithm:

```rust
RateLimiter::keyed(
    Quota::per_second(NonZeroU32::new(rps).unwrap())
        .allow_burst(NonZeroU32::new(burst).unwrap()),
)
```

- Keyed by client IP
- Returns `true` if allowed, `false` if rate limited

### Path Filtering (`filter.rs`)

Uses `globset` for pattern matching:

```rust
// Default blocked patterns
"/.env*,/.git*,/.svn*,/.hg*,/*.sql,/*.bak,/docker-compose*.yml"
```

### Caching (`cache.rs`)

Uses `moka` async cache:

```rust
Cache::builder()
    .max_capacity(config.max_size)
    .time_to_live(Duration::from_secs(3600))  // 1 hour TTL
    .time_to_idle(Duration::from_secs(600))   // 10 min idle
    .build()
```

**Note:** Full response caching disabled in MVP to avoid body buffering complexity.

### Security Headers (`security.rs`)

Default headers added to all responses:

| Header | Value |
|--------|-------|
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains` |
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `X-XSS-Protection` | `1; mode=block` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |

---

## WebSocket (`websocket.rs`)

### Detection

```rust
fn is_websocket_upgrade(req: &Request<Incoming>) -> bool {
    // Check Connection: Upgrade and Upgrade: websocket
}
```

### Handshake

Implements WebSocket handshake with:
- Custom SHA1 implementation (no extra dependencies)
- Base64 encoding for Sec-WebSocket-Accept

### Passthrough

Connects to upstream as WebSocket and maintains bidirectional communication.

**Configured paths:**
```
/_next/webpack-hmr,/ws,/ws/*,/socket.io/*
```

---

## Configuration (`config.rs`)

All configuration via environment variables:

```rust
pub struct Config {
    pub server: ServerConfig,      // Listen addr, HTTP port
    pub upstream: UpstreamConfig,  // Address, timeout
    pub tls: TlsConfig,           // Domains, email, cache dir
    pub rate_limit: RateLimitConfig,
    pub cache: CacheConfig,
    pub security: SecurityConfig,
    pub filter: FilterConfig,
    pub websocket: WebSocketConfig,
    pub log_level: String,
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RPROXY_LISTEN` | `0.0.0.0:443` | Server listen address |
| `RPROXY_HTTP_PORT` | `80` | HTTP redirect port |
| `RPROXY_UPSTREAM` | `127.0.0.1:3000` | Backend address |
| `RPROXY_UPSTREAM_TIMEOUT` | `30` | Timeout in seconds |
| `RPROXY_TLS_DOMAINS` | - | ACME domains (comma-sep) |
| `RPROXY_TLS_EMAIL` | - | Let's Encrypt email |
| `RPROXY_TLS_CACHE_DIR` | `/var/lib/rproxy/certs` | Cert storage |
| `RPROXY_RATE_LIMIT` | `100` | Requests per second |
| `RPROXY_RATE_BURST` | `200` | Burst size |
| `RPROXY_CACHE_ENABLED` | `true` | Enable caching |
| `RPROXY_CACHE_SIZE` | `500` | Max cache entries |
| `RPROXY_HSTS` | `true` | Enable HSTS header |
| `RPROXY_FILTER_ENABLED` | `true` | Enable path filtering |
| `RPROXY_WS_ENABLED` | `true` | Enable WebSocket |
| `RPROXY_LOG_LEVEL` | `info` | Log level |

---

## Error Handling (`error.rs`)

Custom error enum with `thiserror`:

```rust
#[derive(Error, Debug)]
pub enum ProxyError {
    Config(String),
    Io(#[from] io::Error),
    Http(#[from] http::Error),
    Hyper(#[from] hyper::Error),
    Tls(String),
    UpstreamConnection(String),
    UpstreamTimeout,
    WebSocket(String),
    RateLimited,
    PathBlocked(String),
}
```

---

## File Structure

```
apps/server/
├── CLAUDE.md           # This file
├── Cargo.toml          # Dependencies
├── Dockerfile          # Multi-stage build
└── src/
    ├── main.rs         # Entry point
    ├── config.rs       # ENV parsing
    ├── error.rs        # Error types
    ├── server.rs       # HTTP server + TLS
    ├── proxy.rs        # Request forwarding
    ├── websocket.rs    # WS passthrough
    └── middleware/
        ├── mod.rs          # Stack builder
        ├── rate_limit.rs   # Token bucket (governor)
        ├── cache.rs        # HTTP cache (moka)
        ├── security.rs     # Security headers
        └── filter.rs       # Path blocking (globset)
```

---

## Dependencies

```toml
# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP
hyper = { version = "1", features = ["full"] }
hyper-util = { version = "0.1", features = ["full", "tokio"] }
http-body-util = "0.1"

# TLS + ACME
tokio-rustls = "0.26"
rustls = "0.23"
tokio-rustls-acme = "0.7"

# WebSocket
tokio-tungstenite = "0.26"

# Middleware
tower = { version = "0.5", features = ["full"] }
tower-http = { version = "0.6", features = ["compression-br", "compression-gzip"] }
governor = "0.8"  # Rate limiting
moka = { version = "0.12", features = ["future"] }  # Caching

# Utils
globset = "0.4"  # Pattern matching
tracing = "0.1"  # Logging
thiserror = "2"  # Error handling
```

---

## Building

### Development
```bash
cargo build
cargo run
```

### Release
```bash
cargo build --release
# Binary at target/release/rproxy (~4.6 MB on macOS arm64)
```

### Docker
```dockerfile
FROM rust:1.85-alpine AS builder
RUN apk add --no-cache musl-dev
# ... build with musl target

FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rproxy /rproxy
```

---

## Skills

When working on the server, leverage these skills:

- **rust-coder** - Idiomatic Rust patterns
- **rust-debugger** - Debugging compile errors

See `.claude/skills/` for complete skill documentation.
