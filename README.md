# rproxy

A minimal, high-performance reverse proxy written in Rust for self-hosted web applications.

## Features

- **High Performance** - Faster than nginx with 4x less memory usage
- **TLS + ACME** - Automatic Let's Encrypt certificates
- **Rate Limiting** - Token bucket per IP with governor
- **Path Filtering** - Block sensitive files (.env, .git, etc.)
- **WebSocket** - Full passthrough support for HMR
- **Security Headers** - HSTS, X-Frame-Options, etc.
- **Connection Pooling** - Efficient upstream connections

## Quick Start

### Docker

```bash
docker run -d \
  -p 80:80 -p 443:443 \
  -e RPROXY_UPSTREAM=your-app:3000 \
  -e RPROXY_TLS_DOMAINS=example.com \
  -e RPROXY_TLS_EMAIL=admin@example.com \
  -v certs:/var/lib/rproxy/certs \
  abians7/rproxy:latest
```

### Docker Compose

```yaml
services:
  proxy:
    image: abians7/rproxy:latest
    ports:
      - "80:80"
      - "443:443"
    environment:
      - RPROXY_UPSTREAM=app:3000
      - RPROXY_TLS_DOMAINS=example.com,www.example.com
      - RPROXY_TLS_EMAIL=admin@example.com
      - RPROXY_RATE_LIMIT=100
    volumes:
      - certs:/var/lib/rproxy/certs
    depends_on:
      - app

  app:
    image: your-app:latest
    expose:
      - "3000"

volumes:
  certs:
```

## Configuration

All configuration via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `RPROXY_LISTEN` | `0.0.0.0:443` | Listen address |
| `RPROXY_HTTP_PORT` | `80` | HTTP redirect port |
| `RPROXY_UPSTREAM` | `127.0.0.1:3000` | Backend address |
| `RPROXY_UPSTREAM_TIMEOUT` | `30` | Timeout in seconds |
| `RPROXY_TLS_DOMAINS` | - | ACME domains (comma-separated) |
| `RPROXY_TLS_EMAIL` | - | Let's Encrypt email |
| `RPROXY_TLS_CACHE_DIR` | `/var/lib/rproxy/certs` | Certificate storage |
| `RPROXY_RATE_LIMIT` | `100` | Requests per second per IP |
| `RPROXY_RATE_BURST` | `200` | Burst size |
| `RPROXY_CACHE_ENABLED` | `true` | Enable response caching |
| `RPROXY_HSTS` | `true` | Enable HSTS header |
| `RPROXY_FILTER_ENABLED` | `true` | Enable path filtering |
| `RPROXY_BLOCKED_PATHS` | `/.env*,/.git*` | Blocked path patterns |
| `RPROXY_WS_ENABLED` | `true` | Enable WebSocket |
| `RPROXY_WS_PATHS` | `/_next/webpack-hmr,/ws/*` | WebSocket paths |
| `RPROXY_LOG_LEVEL` | `info` | Log level |

## Building from Source

```bash
# Clone
git clone https://github.com/AbianS/rproxy.git
cd rproxy

# Build
cd apps/server
cargo build --release

# Run
./target/release/rproxy
```

## Project Structure

```
├── apps/
│   ├── server/          # Rust proxy server
│   └── docs/            # Documentation site
├── packages/
│   └── benchmarks/      # Benchmark suite
└── .github/
    └── workflows/       # CI/CD pipelines
```

## Documentation

Visit [docs](https://abians.github.io/rproxy) for full documentation.

## License

GPL-3.0 License - see [LICENSE](LICENSE) for details.
