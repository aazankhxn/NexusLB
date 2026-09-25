#!/usr/bin/env bash
set -euo pipefail

echo "==> [1/5] Checking Formatting (cargo fmt --check)..."
cargo fmt --check

echo "==> [2/5] Type Checking Workspace (cargo check --workspace)..."
cargo check --workspace

echo "==> [3/5] Linting Workspace (cargo clippy --workspace --all-targets)..."
cargo clippy --workspace --all-targets

echo "==> [4/5] Running All Workspace Tests (cargo test --workspace)..."
cargo test --workspace

echo "==> [5/5] Building Release Binary (cargo build --release)..."
cargo build --release

echo "===> ALL VERIFICATION CHECKS PASSED SUCCESSFULLY!"
