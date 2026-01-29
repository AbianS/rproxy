#!/bin/bash
# Setup script for benchmark environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "Building benchmark tool..."
cargo build --release

echo "Starting Docker containers..."
docker compose up -d

echo "Waiting for containers to be healthy..."
sleep 10

# Check each container
for container in bench-upstream bench-rproxy bench-nginx bench-traefik bench-caddy bench-envoy; do
    if docker ps --format '{{.Names}}' | grep -q "^${container}$"; then
        echo "  ✓ $container is running"
    else
        echo "  ✗ $container is NOT running"
    fi
done

echo ""
echo "Setup complete! Run benchmarks with:"
echo "  cargo run --release -- run -s sustained"
