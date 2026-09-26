# NexusLB v1.0.0 Production Release Runbook

This runbook outlines the exact sequence of verification steps, artifact builds, and distribution tasks required before uploading and announcing **NexusLB v1.0.0**.

---

## 📋 Pre-Flight Verification Checklist

Execute these checks locally or within the CI environment before tagging `v1.0.0`:

### 1. Codebase Cleanliness & Formatting
Ensure zero formatting regressions across all 18 crates and benchmarks:
```bash
cargo fmt --all -- --check
```

### 2. Zero-Tolerance Compiler & Clippy Auditing
Enforce zero warnings on all targets (including tests and integration benchmarks):
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### 3. Full Test Suite Execution
Run all unit, doc, and integration tests (29 passing tests across routing, zero-allocation buffers, JWT, and failover):
```bash
cargo test --workspace --verbose
```

### 4. Dependency Security Audit
Verify zero known CVEs or supply-chain vulnerabilities in `Cargo.lock`:
```bash
cargo install cargo-audit --locked
cargo audit
```

### 5. Website Static Prerender Validation
Verify the Next.js Apple-designed website and documentation build with zero errors:
```bash
cd website
npm ci
npm run build
cd ..
```

---

## 🏷️ Version Bumping & Git Tagging

### Step 1: Update Root `Cargo.toml`
Update the workspace version in [`Cargo.toml`](file:///Users/aazankhan/Personal/NexusLB/Cargo.toml):
```toml
[workspace.package]
version = "1.0.0"
```

### Step 2: Finalize `CHANGELOG.md`
Promote the `[Unreleased]` section to `[1.0.0] - YYYY-MM-DD` with verified highlights:
- 10 scheduling algorithms (including Adaptive composite scoring and Peak EWMA)
- SIMD-accelerated zero-allocation HTTP streaming dataplane
- Active background health checks and 3-state circuit breaking
- Real-time terminal operator dashboard (`nexuslb top`)
- Next.js Apple-designed documentation center & live simulator

### Step 3: Commit and Tag
```bash
git add .
git commit -m "chore(release): prepare v1.0.0 production release"
git tag -a v1.0.0 -m "NexusLB v1.0.0: High-Performance Sub-Millisecond L7 Reverse Proxy"
git push origin main --tags
```

---

## 📦 Binary Packaging & GitHub Release

The repository includes [`.github/workflows/release.yml`](file:///Users/aazankhan/Personal/NexusLB/.github/workflows/release.yml) which automatically triggers on pushing tag `v*`.

The automated pipeline builds and uploads tarballs with SHA-256 checksums for:
- `nexuslb-linux-amd64.tar.gz` (x86_64-unknown-linux-gnu)
- `nexuslb-linux-arm64.tar.gz` (aarch64-unknown-linux-gnu)
- `nexuslb-darwin-amd64.tar.gz` (x86_64-apple-darwin)
- `nexuslb-darwin-arm64.tar.gz` (aarch64-apple-darwin / Apple Silicon)
- `SHA256SUMS.txt`

### Manual Local Packaging (Alternative)
If releasing manually on macOS Apple Silicon:
```bash
# Compile optimized fat-LTO release binary
cargo build --release -p nexuslb-cli

# Package binary and checksum
mkdir -p dist
cp target/release/nexuslb dist/nexuslb-darwin-arm64
cd dist
tar -czvf nexuslb-v1.0.0-darwin-arm64.tar.gz nexuslb-darwin-arm64
shasum -a 256 nexuslb-v1.0.0-darwin-arm64.tar.gz > SHA256SUMS.txt
cd ..
```

---

## 🐳 Docker Container Release

Build and publish the minimal hardened non-root container image:

```bash
# Build multi-stage Debian bookworm-slim production container
docker build -f deploy/docker/Dockerfile -t ghcr.io/nexuslb/nexuslb:v1.0.0 -t ghcr.io/nexuslb/nexuslb:latest .

# Run sanity smoke test
docker run --rm -d --name nexuslb-test -p 8080:8080 ghcr.io/nexuslb/nexuslb:v1.0.0
curl -I http://localhost:8080/health
docker stop nexuslb-test

# Push to container registry
docker push ghcr.io/nexuslb/nexuslb:v1.0.0
docker push ghcr.io/nexuslb/nexuslb:latest
```

---

## 🦀 Crates.io Publishing Order

Because NexusLB is structured as a Cargo workspace with dependency graphs, crates must be published in topological order:

1. `crates/nexuslb-core`
2. `crates/nexuslb-runtime`
3. `crates/nexuslb-config`
4. `crates/nexuslb-health`
5. `crates/nexuslb-scheduler`
6. `crates/nexuslb-router`
7. `crates/nexuslb-cache`
8. `crates/nexuslb-discovery`
9. `crates/nexuslb-metrics`
10. `crates/nexuslb-observability`
11. `crates/nexuslb-network`
12. `crates/nexuslb-tls`
13. `crates/nexuslb-wasm`
14. `crates/nexuslb-dataplane`
15. `crates/nexuslb-proxy`
16. `crates/nexuslb-api`
17. `crates/nexuslb-tui`
18. `engines/tokio`
19. `crates/nexuslb-cli`

```bash
# Example publish step for each crate:
cargo publish -p nexuslb-core
sleep 15 # Allow crates.io index to sync
cargo publish -p nexuslb-runtime
# ... continue through nexuslb-cli
```

---

## 🌐 Next.js Website & Documentation Deployment

Deploy the Next.js Apple Design website:
- **Vercel / Cloudflare Pages:** Connect GitHub repo, set root directory to `website/`, and trigger production deployment.
- **Docker / Self-Hosted:** Run `npm run build && npm start` behind NexusLB.
