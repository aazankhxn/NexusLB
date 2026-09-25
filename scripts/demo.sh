#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT_DIR}"

echo "=========================================================="
echo "          NEXUSLB LIVE PRODUCTION DEMO                    "
echo "=========================================================="

cleanup() {
    echo ""
    echo "[Cleanup] Stopping mock backends and NexusLB..."
    kill $(jobs -p) 2>/dev/null || true
    echo "[Cleanup] Done."
}
trap cleanup EXIT

# 1. Start High-Performance Async Mock Backend 1
./target/release/mock-server 9001 api-node-1 &
PID_B1=$!
echo "[1/6] Mock Backend 1 (Rust Async) started on 127.0.0.1:9001 (PID $PID_B1)"

# 2. Start High-Performance Async Mock Backend 2
./target/release/mock-server 9002 api-node-2 &
PID_B2=$!
echo "[2/6] Mock Backend 2 (Rust Async) started on 127.0.0.1:9002 (PID $PID_B2)"

sleep 0.5

# 3. Start NexusLB in Release Mode
echo "[3/6] Starting NexusLB (Release Profile, LTO) on 0.0.0.0:8080..."
./target/release/nexuslb start --config nexuslb.yaml &
PID_NEXUS=$!
sleep 1

# 4. Send Individual HTTP Requests (Demonstrating Adaptive / Multi-node balancing)
echo ""
echo "[4/6] Sending 6 sequential HTTP requests to NexusLB (0.0.0.0:8080):"
for i in {1..6}; do
    RESPONSE=$(curl -s http://127.0.0.1:8080/)
    echo "  Request $i => $RESPONSE"
done

# 5. Inspect Admin API & Metrics
echo ""
echo "[5/6] Inspecting Real-Time Backend Latency & Statistics (Admin API 127.0.0.1:9091/backends):"
curl -s http://127.0.0.1:9091/backends | head -n 35

echo ""
echo "[5b] Scraping Prometheus Metrics (127.0.0.1:9090/metrics):"
curl -s http://127.0.0.1:9090/metrics | grep -E "nexuslb_requests_total|nexuslb_connections_total|nexuslb_active_connections|nexuslb_bytes"

# 6. Run Concurrency Benchmark Load Test
echo ""
echo "[6/6] Executing NexusLB Concurrency Load Test (100 concurrent workers, 3 seconds)..."
./target/release/nexuslb-bench run --target 127.0.0.1:8080 --concurrency 100 --duration-secs 3

echo ""
echo "=========================================================="
echo "          DEMONSTRATION COMPLETED SUCCESSFULLY!           "
echo "=========================================================="
