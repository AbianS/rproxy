# rproxy Benchmarks

Comprehensive benchmark suite for comparing rproxy against other popular reverse proxies:

- **rproxy** - Our Rust-based reverse proxy
- **nginx** - Industry standard, C-based
- **traefik** - Cloud-native, Go-based
- **caddy** - Modern, Go-based with automatic HTTPS
- **envoy** - High-performance, C++-based (service mesh)

## Quick Start

```bash
# Start all proxy containers
cargo run --release -- up -d

# Run a benchmark (all proxies)
cargo run --release -- run -s sustained

# Compare specific proxies
cargo run --release -- run -s sustained -t rproxy
cargo run --release -- run -s sustained -t nginx

# Compare results
cargo run --release -- compare results/result1.json results/result2.json

# Generate report
cargo run --release -- report -i results -o benchmark-report.md

# Stop containers
cargo run --release -- down
```

## Scenarios

| Scenario | Description | Duration | Load |
|----------|-------------|----------|------|
| `baseline` | Minimal load for base latency | 60s | 1 req/s |
| `sustained` | Steady-state performance | 5 min | 1k req/s |
| `burst` | Traffic spike handling | 2 min | 10k bursts |
| `stress` | Find breaking point | 10 min max | Ramp to 50k |
| `memory` | Memory leak detection | 30 min | 500 req/s |
| `concurrent` | High connection count | 2 min | 1k connections |
| `api-workload` | Mixed API traffic | 3 min | 1k req/s mixed |

## Resource Limits

All proxies run with identical resource constraints:
- **CPU**: 1 core
- **Memory**: 256 MB

## Metrics Collected

- **Throughput**: Requests/second, success/failure counts
- **Latency**: P50, P90, P95, P99, P99.9, min, max, mean
- **Memory**: Idle, peak, average (MB)
- **CPU**: Peak, average (%)
- **Network**: RX/TX bytes
- **Errors**: Timeout, connection refused, 4xx, 5xx

## Custom Scenarios

Create a TOML file:

```toml
name = "my-scenario"
description = "Custom benchmark"

[load]
target_rps = 2000
concurrency = 100
duration_secs = 120
warmup_secs = 10

[request]
method = "POST"
path = "/api/data"
body_size = 4096
keep_alive = true
compression = true
```

Run it:

```bash
cargo run --release -- run -s path/to/my-scenario.toml
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Benchmark Runner                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   Workload  │  │   Metrics   │  │   Report    │          │
│  │  Generator  │  │  Collector  │  │  Generator  │          │
│  └──────┬──────┘  └──────┬──────┘  └─────────────┘          │
└─────────┼────────────────┼──────────────────────────────────┘
          │                │
          ▼                ▼
┌─────────────────────────────────────────────────────────────┐
│                      Docker Network                         │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐     │
│  │ rproxy │ │ nginx  │ │traefik │ │ caddy  │ │ envoy  │     │
│  │ :8080  │ │ :8081  │ │ :8082  │ │ :8083  │ │ :8084  │     │
│  └────┬───┘ └────┬───┘ └────┬───┘ └────┬───┘ └────┬───┘     │
│       │          │          │          │          │          │
│       └──────────┴──────────┴──────────┴──────────┘          │
│                            │                                 │
│                     ┌──────▼──────┐                          │
│                     │  upstream   │                          │
│                     │   :80       │                          │
│                     └─────────────┘                          │
└─────────────────────────────────────────────────────────────┘
```

## Output

Results are saved as JSON files:

```json
{
  "run_id": "20260129-143022-sustained-rproxy",
  "timestamp": "2026-01-29T14:30:22Z",
  "proxy": "rproxy",
  "scenario": "sustained",
  "results": {
    "throughput": {
      "requests_per_second": 985.5,
      "total_requests": 295650,
      "successful": 295600,
      "failed": 50
    },
    "latency_ms": {
      "p50": 8.2,
      "p95": 24.1,
      "p99": 45.3
    },
    "memory_mb": {
      "idle": 12,
      "peak": 48,
      "average": 32
    }
  }
}
```

## Commands

```
rproxy-bench

Commands:
  run      Run a benchmark scenario
  list     List available scenarios
  compare  Compare two benchmark results
  show     Display a result file
  up       Start proxy containers
  down     Stop proxy containers
  stats    Show container resource usage
  report   Generate comparison report (md/html/json)

Options:
  --log-level <LEVEL>  Log level (default: info)
```

## Requirements

- Docker with Compose v2
- Rust 1.75+ (for building the benchmark tool)

## Building

```bash
cd packages/benchmarks
cargo build --release
```

The benchmark runner binary will be at `target/release/rproxy-bench`.
