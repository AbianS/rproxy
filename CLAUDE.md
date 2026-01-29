# rproxy - Minimal Reverse Proxy

> **Context Architecture Note**: This is the **root context** file for the rproxy project.
> For component-specific context, see:
> - Server (Rust backend): `apps/server/CLAUDE.md`
> - Documentation (Next.js): `apps/docs/CLAUDE.md`

## Project Vision

rproxy is an ultra-lightweight, high-performance reverse proxy designed for self-hosted web applications (Next.js, etc.) in Docker/VPS environments. The key differentiator is **minimal resource footprint** while providing essential production features.

**Target Metrics:**
- **Memory**: ~10-15 MB RAM
- **Binary**: ~5 MB (macOS arm64), ~6-12 MB Docker image
- **Single binary**: No runtime dependencies

## Architecture

```
┌─────────────────┐     ┌─────────────────────┐     ┌─────────────────┐
│    Internet     │────▶│      rproxy         │────▶│    Backend      │
│   (Clients)     │     │  (Rust/hyper 1.x)   │     │  (Next.js, etc) │
└─────────────────┘     └─────────────────────┘     └─────────────────┘
                                  │
                        ┌─────────┴─────────┐
                        │   Middleware      │
                        ├───────────────────┤
                        │ • TLS (ACME)      │
                        │ • Rate Limiting   │
                        │ • Path Filtering  │
                        │ • Caching         │
                        │ • Security Headers│
                        │ • WebSocket       │
                        │ • Compression     │
                        └───────────────────┘
```

## Tech Stack

### Server (`apps/server`)
- **Language**: Rust (2024 edition, 1.85+)
- **HTTP**: hyper 1.x + hyper-util
- **Async Runtime**: Tokio
- **TLS**: rustls + tokio-rustls-acme (Let's Encrypt)
- **Rate Limiting**: governor (token bucket)
- **Caching**: moka (async cache)
- **Compression**: tower-http (Brotli + Gzip)
- **WebSocket**: tokio-tungstenite

## Directory Structure

```
proxy/
├── CLAUDE.md              # This file - project culture
├── apps/
│   └── server/            # Rust reverse proxy
│       ├── CLAUDE.md      # Server-specific context
│       ├── Cargo.toml
│       ├── Dockerfile
│       └── src/
│           ├── main.rs        # Entry point + logging
│           ├── config.rs      # ENV var parsing
│           ├── error.rs       # Custom errors
│           ├── server.rs      # HTTP server + TLS
│           ├── proxy.rs       # Request forwarding
│           ├── websocket.rs   # WS passthrough
│           └── middleware/
│               ├── mod.rs         # Stack builder
│               ├── rate_limit.rs  # Token bucket
│               ├── cache.rs       # HTTP cache
│               ├── security.rs    # Headers
│               └── filter.rs      # Path blocking
├── .claude/
│   ├── skills/            # AI context modules
│   └── commands/          # Custom slash commands
├── turbo.json             # Monorepo config
└── package.json           # Workspace root
```

## Key Features

| Feature | Description |
|---------|-------------|
| **TLS + ACME** | Automatic Let's Encrypt certificates |
| **HTTP/2** | Auto-detection over TLS |
| **Rate Limiting** | Token bucket per IP (governor) |
| **Caching** | Static assets with moka |
| **Security Headers** | HSTS, X-Frame-Options, etc. |
| **Path Filtering** | Block .env, .git, sensitive files |
| **WebSocket** | Passthrough for HMR |
| **Compression** | Brotli + Gzip |

## Configuration

rproxy uses **environment variables only** (no config files) for Docker/K8s compatibility.

| Variable | Default | Description |
|----------|---------|-------------|
| `RPROXY_LISTEN` | `0.0.0.0:443` | Listen address |
| `RPROXY_HTTP_PORT` | `80` | HTTP redirect port |
| `RPROXY_UPSTREAM` | `127.0.0.1:3000` | Backend address |
| `RPROXY_TLS_DOMAINS` | - | ACME domains (comma-sep) |
| `RPROXY_TLS_EMAIL` | - | Let's Encrypt email |
| `RPROXY_RATE_LIMIT` | `100` | Requests/second |
| `RPROXY_CACHE_ENABLED` | `true` | Enable caching |
| `RPROXY_HSTS` | `true` | Enable HSTS |

## Code Conventions

### Rust
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Error handling with `thiserror`
- Prefer `async/await` over blocking code
- Use `tracing` for logging

### General
- Commit messages: `type: description` (feat, fix, docs, refactor, test)
- Keep functions small and focused
- Avoid over-engineering - MVP first

## Development Workflow

### Local Development
```bash
cd apps/server

# Run without TLS
RPROXY_UPSTREAM=127.0.0.1:3000 RPROXY_TLS_DOMAINS="" cargo run

# Build release
cargo build --release
```

### Docker
```bash
docker build -t rproxy apps/server
docker run -e RPROXY_UPSTREAM=host.docker.internal:3000 rproxy
```

## Request Flow

```
Client → TLS (rustls) → Rate Limit → Path Filter → Cache Check
                                                        ↓
                                         [Cache Hit] → Response
                                         [Cache Miss] ↓
                                    WebSocket? → WS Passthrough
                                         ↓
                                    HTTP Forward → Backend
                                         ↓
                              Security Headers + Compression
                                         ↓
                                      Response
```

## Skills System

This project uses the Agent Skills standard for AI-assisted development. Skills are in `.claude/skills/`.

### Available Skills

**Generic Skills:**
- **skill-creator** - Guidelines for creating new skills
- **code-recall** - Persistent memory across sessions

**Project Skills** (to be created as needed):
- **rust-proxy** - Proxy patterns, middleware, hyper usage

### Skill Usage

Skills automatically activate based on:
- Working in relevant scope (server → rust skills)
- Trigger conditions in skill frontmatter
- Explicit user request

## Memory Management (code-recall MCP)

Use the code-recall MCP to maintain persistent memory across sessions.

### Session Start
```
mcp__code-recall__get_briefing({ focus_areas: ["current task keywords"] })
```

### After Important Decisions
```
mcp__code-recall__store_observation({
  category: "decision" | "pattern" | "warning" | "learning",
  content: "What was decided",
  rationale: "Why this approach was chosen",
  tags: ["relevant", "tags"]
})
```
