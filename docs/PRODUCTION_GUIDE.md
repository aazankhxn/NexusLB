# NexusLB Production Operations & Tuning Guide

This guide covers system optimization, kernel tuning, containerization, systemd supervision, and Prometheus monitoring for deploying NexusLB in mission-critical high-throughput environments.

---

## 1. Operating System & Kernel Tuning (Linux)

To sustain **100,000+ requests per second** per node without dropped packets or socket exhaustion, apply the following sysctl settings:

Add to `/etc/sysctl.d/99-nexuslb.conf`:

```ini
# Increase system-wide file descriptor limit
fs.file-max = 2097152

# Increase max pending socket backlog connections
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535

# Enable fast port reuse for outbound connections
net.ipv4.tcp_tw_reuse = 1
net.ipv4.ip_local_port_range = 1024 65535

# Increase socket buffer sizes
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216

# Increase network interface packet queue
net.core.netdev_max_backlog = 100000
```

Apply immediately:
```bash
sudo sysctl -p /etc/sysctl.d/99-nexuslb.conf
```

### Process Limits (`/etc/security/limits.d/99-nexuslb.conf`)
```ini
nexuslb soft nofile 1048576
nexuslb hard nofile 1048576
```

---

## 2. Systemd Service Deployment

Create `/etc/systemd/system/nexuslb.service`:

```ini
[Unit]
Description=NexusLB High-Performance Adaptive Load Balancer
Documentation=https://github.com/nexuslb/nexuslb
After=network.target

[Service]
Type=simple
User=nexuslb
Group=nexuslb
LimitNOFILE=1048576
ExecStart=/usr/local/bin/nexuslb start --config /etc/nexuslb/nexuslb.yaml
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=2s
AmbientCapabilities=CAP_NET_BIND_SERVICE CAP_NET_ADMIN

[Install]
WantedBy=multi-user.target
```

Reload and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now nexuslb
```

To reload configuration without downtime:
```bash
sudo systemctl reload nexuslb
```

---

## 3. Prometheus & Grafana Monitoring

NexusLB exposes Prometheus metrics on port `9090` by default.

### Key Metrics to Monitor

| Metric Name | Type | Description | Alert Threshold |
| :--- | :--- | :--- | :--- |
| `nexuslb_requests_total` | Counter | Total requests handled across workers | Rate drops unexpectedly |
| `nexuslb_active_connections` | Gauge | Currently open client connections | Exceeds 80% of max_conn |
| `nexuslb_latency_p99_us` | Gauge | 99th percentile end-to-end latency in µs | > 10,000 µs (10ms) |
| `nexuslb_backend_errors_total`| Counter | Total upstream connection or 5xx errors | Rate > 0.5% of total |
| `nexuslb_dropped_connections`| Counter | Dropped requests due to rate limits | Sudden spike |

### Sample Prometheus Scrape Config
```yaml
scrape_configs:
  - job_name: "nexuslb"
    scrape_interval: 5s
    static_configs:
      - targets: ["10.0.0.1:9090", "10.0.0.2:9090"]
```

---

## 4. Docker & Container Deployment

A minimal multi-stage Dockerfile:

```dockerfile
FROM rust:1.80-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY . .
RUN cargo build --release

FROM alpine:3.20
RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/release/nexuslb /usr/local/bin/nexuslb
EXPOSE 80 443 9090 9091
ENTRYPOINT ["/usr/local/bin/nexuslb", "start", "--config", "/etc/nexuslb/nexuslb.yaml"]
```
