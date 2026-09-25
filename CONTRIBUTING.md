# Contributing to NexusLB

Thank you for your interest in contributing to NexusLB!

## Code Quality Standards

Before submitting a Pull Request, ensure that:

1. **Formatting**:
   ```bash
   cargo fmt --check
   ```
2. **Type Checking**:
   ```bash
   cargo check --workspace
   ```
3. **Clippy Lints**:
   ```bash
   cargo clippy --workspace --all-targets
   ```
4. **Tests**:
   ```bash
   cargo test --workspace
   ```
5. **No Performance Regressions**:
   Ensure performance-sensitive changes are measured with `cargo bench -p nexuslb-benchmarks`.
   Every optimization must answer: *"What bottleneck does this remove?"*

## Architecture Rules

- Maintain clean module boundaries across the workspace crates.
- Never add locks or heap allocations to the request hot path.
- Keep dependencies minimal and evaluated for their impact on binary size and compilation time.
