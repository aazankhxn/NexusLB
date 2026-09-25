#!/usr/bin/env bash
set -euo pipefail

echo "=== System Info ==="
uname -a
echo "CPU Cores: $(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo unknown)"
echo "Memory: $(sysctl -n hw.memsize 2>/dev/null || free -m 2>/dev/null || echo unknown)"
echo "Rust Version: $(rustc --version)"
echo "Cargo Version: $(cargo --version)"
