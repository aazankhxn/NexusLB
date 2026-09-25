# Stage 1: Build binary with maximum optimizations
FROM rust:1.80-bullseye AS builder

WORKDIR /usr/src/nexuslb

# Copy dependency manifests and source
COPY . .

# Compile release binary with fat LTO and stripped symbols
RUN cargo build --release -p nexuslb-cli

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Run as non-privileged user for security
RUN useradd -u 10001 -U -m -s /bin/false nexuslb

COPY --from=builder /usr/src/nexuslb/target/release/nexuslb /usr/local/bin/nexuslb
COPY configs/nexuslb.yaml /etc/nexuslb/nexuslb.yaml

RUN chown -R nexuslb:nexuslb /etc/nexuslb

USER nexuslb

EXPOSE 8080 9090 9091

ENTRYPOINT ["nexuslb", "start", "--config", "/etc/nexuslb/nexuslb.yaml"]
