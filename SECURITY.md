# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in NexusLB, please do not file a public issue.
Instead, report it directly to the core maintainers via security advisory or email.

All reports will be acknowledged within 48 hours, followed by a security audit and patch release.

## Memory & Network Safety

NexusLB is implemented entirely in safe Rust, guaranteeing:
- Freedom from buffer overflows, use-after-free, and double-free vulnerabilities.
- Data race freedom across multi-core workers.
- Strict HTTP and configuration input validation to prevent injection or resource exhaustion attacks.
