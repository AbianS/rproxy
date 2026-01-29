#!/bin/bash
# Run full benchmark comparison across all proxies

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$PROJECT_DIR/results"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

cd "$PROJECT_DIR"

# Default scenario
SCENARIO="${1:-sustained}"

echo "═══════════════════════════════════════════════════════"
echo "  Reverse Proxy Benchmark Comparison"
echo "  Scenario: $SCENARIO"
echo "  Timestamp: $TIMESTAMP"
echo "═══════════════════════════════════════════════════════"
echo ""

# Ensure containers are running
echo "Checking Docker containers..."
docker compose up -d
sleep 5

# Create results directory
mkdir -p "$RESULTS_DIR/$TIMESTAMP"

# Run benchmarks for each proxy
for proxy in rproxy nginx traefik caddy envoy; do
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Benchmarking: $proxy"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    cargo run --release -- run -s "$SCENARIO" -t "$proxy" -o "$RESULTS_DIR/$TIMESTAMP"

    # Cool down between tests
    echo "Cooling down for 10 seconds..."
    sleep 10
done

# Generate comparison report
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Generating Report"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cargo run --release -- report \
    -i "$RESULTS_DIR/$TIMESTAMP" \
    -o "$RESULTS_DIR/$TIMESTAMP/report.md" \
    -f markdown

echo ""
echo "═══════════════════════════════════════════════════════"
echo "  Benchmark Complete!"
echo "  Results: $RESULTS_DIR/$TIMESTAMP/"
echo "  Report:  $RESULTS_DIR/$TIMESTAMP/report.md"
echo "═══════════════════════════════════════════════════════"
