#!/usr/bin/env bash
set -euo pipefail

echo "=========================================================="
echo "          NEXUSLB BENCHMARK LAB RUNNER                    "
echo "=========================================================="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ROOT_DIR}"

echo "[1/3] Gathering System Hardware & Environment..."
cargo run --release -p nexuslb-benchmarks --bin nexuslb-bench -- system-info

echo "[2/3] Running Criterion Microbenchmarks for Schedulers & Hot Path..."
cargo bench -p nexuslb-benchmarks

echo "[3/3] Benchmark complete. Artifacts written to target/criterion"
