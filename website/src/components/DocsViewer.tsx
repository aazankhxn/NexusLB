"use client";

import { useState, useMemo } from "react";
import {
  Terminal,
  Cpu,
  Settings,
  BarChart3,
  Shield,
  Activity,
  Server,
  Layers,
  Search,
  Copy,
  Check,
  ChevronRight,
  ArrowRight,
  ArrowLeft,
  ExternalLink,
  Zap,
  Lock,
  Flame,
  Radio,
  Sliders,
  CheckCircle2,
  AlertTriangle,
  Info,
} from "lucide-react";

interface SubSection {
  id: string;
  title: string;
  desc?: string;
  code?: string;
  codeLang?: string;
  bullets?: string[];
  callout?: {
    type: "note" | "tip" | "important" | "security";
    text: string;
  };
  table?: {
    headers: string[];
    rows: string[][];
  };
}

interface DocCategory {
  id: string;
  title: string;
  icon: React.ReactNode;
  summary: string;
  subsections: SubSection[];
}

export function DocsViewer() {
  const [activeCategory, setActiveCategory] = useState<string>("getting-started");
  const [activeSubId, setActiveSubId] = useState<string>("installation");
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  const copyCode = (code: string, id: string) => {
    navigator.clipboard.writeText(code);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const docModules: DocCategory[] = useMemo(
    () => [
      {
        id: "getting-started",
        title: "1. Getting Started",
        icon: <Terminal size={17} />,
        summary: "Zero-dependency setup, binary compilation, and running your first proxy in under 60 seconds.",
        subsections: [
          {
            id: "installation",
            title: "Installation & System Requirements",
            desc: "NexusLB runs on macOS (Apple Silicon & Intel) and Linux (x86_64, aarch64, ARMv7). It compiles into a completely self-contained single binary with zero external runtime dependencies.",
            bullets: [
              "Rust 1.80+ (stable toolchain recommended)",
              "Linux kernel 5.1+ required for io_uring and AF_XDP engines; Tokio engine runs universally on all platforms including macOS",
              "OpenSSL is NOT required (native pure-Rust rustls cryptographic engine)",
            ],
            codeLang: "bash",
            code: `# Clone and compile with Maximum Fat LTO optimization
git clone https://github.com/Aazann/NexusLB.git
cd NexusLB

# Build optimized release binary
cargo build --release -p nexuslb-cli

# Verify version and target architecture
./target/release/nexuslb version`,
            callout: {
              type: "tip",
              text: "For production deployments on Linux, compiling with 'RUSTFLAGS=\"-C target-cpu=native\"' enables SIMD vectorized httparse instructions (AVX2/NEON), boosting header parsing throughput by 12–18%.",
            },
          },
          {
            id: "first-run",
            title: "60-Second Quickstart",
            desc: "Bootstrap a production-ready Layer 7 reverse proxy routing between local microservices.",
            codeLang: "bash",
            code: `# 1. Start two mock backend HTTP servers on ports 8081 and 8082
python3 -m http.server 8081 &
python3 -m http.server 8082 &

# 2. Start NexusLB using the default configuration
./target/release/nexuslb start --config nexuslb.yaml

# 3. Test HTTP proxy forwarding with active metrics
curl -i http://localhost:8080/

# 4. In a separate terminal, launch the live operator dashboard
./target/release/nexuslb top`,
          },
          {
            id: "cli-reference",
            title: "CLI Command Reference",
            desc: "The nexuslb binary exposes subcommands for running, inspecting, and hot-reloading active daemon processes.",
            table: {
              headers: ["Command", "Flags / Options", "Description"],
              rows: [
                ["nexuslb start", "--config <PATH>, --engine <tokio|io-uring|xdp|auto>, --workers <N>", "Starts proxy server with specified engine"],
                ["nexuslb top", "--admin-addr <ADDR> (default: 127.0.0.1:9091)", "Interactive 10 Hz curses operator terminal"],
                ["nexuslb reload", "--admin-addr <ADDR>, --token <TOKEN>", "Triggers sub-microsecond atomic config swap"],
                ["nexuslb status", "--admin-addr <ADDR>", "Queries running status and active upstream pools"],
                ["nexuslb version", "None", "Outputs binary build target, commit, and engines"],
              ],
            },
          },
        ],
      },
      {
        id: "architecture",
        title: "2. Dataplane Architecture",
        icon: <Cpu size={17} />,
        summary: "Zero-allocation byte-slice streaming, lock-free ArrayQueue buffer pools, and multi-engine abstractions.",
        subsections: [
          {
            id: "zero-alloc",
            title: "Zero-Allocation Ingress Pipeline",
            desc: "Traditional C and Go proxies suffer from per-request malloc/free heap allocations, inducing lock contention and GC stop-the-world pauses. NexusLB uses pre-allocated ring buffers and SIMD httparse byte slices.",
            bullets: [
              "Lock-free BufferPool pre-allocates 64 KB buffers using crossbeam ArrayQueue",
              "HTTP header parsing creates zero string allocations by referencing slices of the raw ingress buffer",
              "Eliminates per-request syscalls, dropping memory allocation latency from ~148 ns to ~11.4 ns (~13x faster)",
            ],
            codeLang: "rust",
            code: `// Hot-path buffer acquisition from lock-free pool
let buffer = buffer_pool.acquire().unwrap_or_else(|| {
    // Dynamic fallback only if load spikes beyond pre-allocated capacity
    vec![0u8; BUFFER_CAPACITY]
});

// SIMD vectorized slice parsing (no heap strings created)
let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
let mut req = httparse::Request::new(&mut headers);
let status = req.parse(&buffer)?;`,
          },
          {
            id: "connection-pool",
            title: "Upstream Keep-Alive Connection Pooling",
            desc: "TCP three-way handshakes introduce 1–3 RTT delays. NexusLB maintains warm keep-alive connection pools to every upstream backend, reusing established TCP sockets across client sessions.",
            table: {
              headers: ["Feature", "NexusLB", "Standard Reverse Proxy"],
              rows: [
                ["Handshake Overhead", "0 ms on warm pool", "1–3 RTT per connection"],
                ["Keep-Alive Recycling", "Automatic with idle reaper", "Ad-hoc / Manual"],
                ["Health Verification", "Pre-flight socket readiness check", "Fails on broken pipe"],
                ["Pool Scaling", "Lock-free bounded ArrayQueue", "Global Mutex locks"],
              ],
            },
          },
          {
            id: "multi-engine",
            title: "Multi-Engine Dataplane Architecture",
            desc: "NexusLB abstracts OS-specific network capabilities behind the Engine trait. Choose between universal Tokio, Linux io_uring, or kernel-bypass AF_XDP.",
            callout: {
              type: "important",
              text: "Tokio is default and portable across all operating systems. Linux io_uring eliminates kernel-userspace context switches via submission and completion queues (SQ/CQ).",
            },
          },
        ],
      },
      {
        id: "configuration",
        title: "3. Configuration Reference",
        icon: <Settings size={17} />,
        summary: "Comprehensive guide to nexuslb.yaml syntax, virtual hosts, routing prefixes, and filters.",
        subsections: [
          {
            id: "yaml-schema",
            title: "Full nexuslb.yaml Schema",
            desc: "A production-grade NexusLB configuration is declarative, human-readable, and strongly typed.",
            codeLang: "yaml",
            code: `version: "1"

server:
  workers: auto              # Number of worker threads (default: CPU cores)
  engine: tokio              # tokio | io_uring | xdp | auto
  max_connections: 50000     # Global file descriptor connection cap
  tcp_nodelay: true          # Disable Nagle's algorithm for minimum latency
  reuse_port: true           # Kernel SO_REUSEPORT multi-queue dispatch

listeners:
  - address: "0.0.0.0:8080"
    protocol: http1
    routes:
      - path_prefix: "/api/v1"
        pool: api-pool
      - path_prefix: "/static"
        pool: static-pool
      - path_prefix: "/"
        pool: default-pool

pools:
  api-pool:
    algorithm: adaptive      # adaptive | ewma | p2c | least_conn | ketama | round_robin
    backends:
      - id: node-01
        address: "10.0.1.10:8080"
        weight: 100
      - id: node-02
        address: "10.0.1.11:8080"
        weight: 100
      - id: node-03
        address: "10.0.1.12:8080"
        weight: 50

health_check:
  enabled: true
  interval: 5s
  timeout: 1s
  unhealthy_threshold: 3
  healthy_threshold: 2
  http_path: "/health"

circuit_breaker:
  enabled: true
  consecutive_errors: 5
  recovery_time: 15s

admin:
  listen: "127.0.0.1:9091"
  auth_token: "secret-bearer-token"`,
          },
          {
            id: "routing-rules",
            title: "Path Rewriting & Header Mutation",
            desc: "Intercept, strip, or inject headers before request dispatch to backend servers.",
            codeLang: "yaml",
            code: `routes:
  - path_prefix: "/legacy-api"
    strip_prefix: "/legacy-api"
    pool: new-backend-pool
    headers:
      inject:
        X-Forwarded-Proto: "https"
        X-Proxied-By: "NexusLB-v0.0.1"
      remove:
        - "X-Internal-Debug-Token"`,
          },
        ],
      },
      {
        id: "algorithms",
        title: "4. The 10 Algorithms",
        icon: <BarChart3 size={17} />,
        summary: "Mathematical models, selection formulas, and empirical performance tradeoffs of all 10 schedulers.",
        subsections: [
          {
            id: "adaptive-algo",
            title: "Adaptive Composite Scoring (Recommended)",
            desc: "NexusLB's flagship load balancing algorithm. Unlike naive Round Robin or Least Connections which only inspect static numbers, Adaptive balances real-time EWMA response latency, queue depth, consecutive errors, and circuit breaker state.",
            codeLang: "text",
            code: `Composite Score Formulation:
Score = (EWMA_Latency_ms × W_lat) + ((Active_Conns / Weight) × 15 × W_load) + ((Errors × 25 + ErrorRate% × 10) × W_err) + State_Penalty

Where:
- W_lat = 0.50 (Latency weight)
- W_load = 0.35 (Connection load weight)
- W_err = 0.15 (Error weight)
- State_Penalty = 0 (Up), 200 (Starting), 10,000 (Quarantined)

Target with LOWEST score is selected for the next incoming request.`,
            callout: {
              type: "tip",
              text: "In production environments with variable payload sizes or background GC pauses (Java/Node.js/Go), Adaptive prevents traffic congestion by routing requests away from stalling instances before health checks even trigger.",
            },
          },
          {
            id: "ketama-algo",
            title: "Consistent Hashing (Ketama Ring)",
            desc: "Distributes requests across a virtual 32-bit Murmur3/FNV-1a ring with 160 virtual nodes per physical host. Ensures cache affinity for session tokens and user IDs.",
            codeLang: "rust",
            code: `// Virtual node ring lookup: O(log N) binary search
let hash = murmur3_32(client_key);
let target_node = ring.range(hash..).next()
    .unwrap_or_else(|| ring.iter().next().unwrap());`,
          },
          {
            id: "p2c-algo",
            title: "Power of Two Random Choices (P2C)",
            desc: "P2C samples two candidate nodes at random and selects the one with the lowest active connection count. It achieves near-optimal load distribution with O(1) time complexity, avoiding global lock contention.",
          },
        ],
      },
      {
        id: "admin-api",
        title: "5. Admin REST API & TUI",
        icon: <Activity size={17} />,
        summary: "REST API endpoints, Prometheus metric schemas, and real-time terminal operator dashboard.",
        subsections: [
          {
            id: "endpoints-list",
            title: "REST Management Endpoints",
            desc: "NexusLB exposes a non-blocking administrative HTTP service on port 9091 for orchestration platforms.",
            table: {
              headers: ["Method & Route", "Purpose", "Example Payload / Output"],
              rows: [
                ["GET /health", "Liveness probe", "{\"status\": \"healthy\", \"uptime_s\": 4920}"],
                ["GET /ready", "Readiness check", "{\"ready\": true, \"healthy_nodes\": 4}"],
                ["GET /backends", "Full cluster metrics", "Detailed JSON latency & conns per node"],
                ["POST /backends/:id/drain", "Graceful backend drain", "{\"timeout_seconds\": 60}"],
                ["POST /reload", "Zero-downtime hot reload", "ArcSwap < 1 µs configuration swap"],
                ["GET /metrics", "Prometheus scraper", "Standard OpenMetrics text format"],
              ],
            },
          },
          {
            id: "tui-dashboard",
            title: "Interactive Terminal Operator (nexuslb top)",
            desc: "Inspect live request rates, connection counts, and percentile latencies at 10 Hz directly in your shell.",
            codeLang: "bash",
            code: `# Run operator dashboard connected to local daemon
./target/release/nexuslb top

# Keyboard Shortcuts:
# [q]      Quit dashboard
# [Tab]    Cycle between backend pools
# [d]      Drain currently highlighted node
# [r]      Trigger manual configuration reload
# [Space]  Sort by Throughput / Latency / Conns`,
          },
        ],
      },
      {
        id: "security",
        title: "6. Security & Hardening",
        icon: <Shield size={17} />,
        summary: "Memory safety guarantees, TLS 1.3 PrefixedStream termination, JWT Bearer verification, and rate limiting.",
        subsections: [
          {
            id: "memory-safety",
            title: "Mathematical Memory Safety (Safe Rust)",
            desc: "NexusLB's core dataplane is written in safe Rust. The compiler's borrow checker mathematically guarantees the absence of common vulnerabilities that plague legacy C proxies.",
            bullets: [
              "0 Buffer Overflows or Off-by-one pointer errors",
              "0 Use-after-free or double-free memory corruption bugs",
              "0 Data races across multi-threaded per-core worker threads",
              "Compiler-enforced panic = 'abort' preventing unwinding attack vectors",
            ],
          },
          {
            id: "tls-sni",
            title: "TLS 1.3 Termination & Dynamic SNI",
            desc: "Hardware-accelerated TLS termination powered by pure-Rust rustls. Supports dynamic SNI certificate selection without dropping in-flight TLS handshakes.",
            codeLang: "yaml",
            code: `tls:
  enabled: true
  certificates:
    - domain: "api.example.com"
      cert_file: "/etc/ssl/api.crt"
      key_file: "/etc/ssl/api.key"
    - domain: "*.example.com"
      cert_file: "/etc/ssl/wildcard.crt"
      key_file: "/etc/ssl/wildcard.key"
  alpn: ["h2", "http/1.1"]`,
          },
        ],
      },
      {
        id: "production",
        title: "7. Production Runbook",
        icon: <Server size={17} />,
        summary: "Linux kernel socket tuning, systemd sandboxing, Docker containers, and Kubernetes deployment YAML.",
        subsections: [
          {
            id: "kernel-tuning",
            title: "Linux Kernel sysctl.conf Tuning",
            desc: "High-throughput proxies handling >100,000 req/s require kernel TCP buffer tuning to prevent SYN flood drops and ephemeral port exhaustion.",
            codeLang: "ini",
            code: `# /etc/sysctl.d/99-nexuslb.conf
# Increase maximum socket listen backlog queue
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535

# Expand ephemeral port range for upstream connection pooling
net.ipv4.ip_local_port_range = 1024 65535

# Enable fast TIME_WAIT socket recycling
net.ipv4.tcp_tw_reuse = 1

# Enlarge TCP receive and transmit buffers (16 MB max)
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216

# Apply changes immediately
# sudo sysctl --system`,
          },
          {
            id: "systemd-unit",
            title: "Hardened Systemd Service Unit",
            desc: "Runs NexusLB under an unprivileged user with security sandboxing, capabilities isolation, and automatic restart.",
            codeLang: "ini",
            code: `[Unit]
Description=NexusLB Layer 7 High-Performance Reverse Proxy
After=network.target

[Service]
Type=simple
User=nexuslb
Group=nexuslb
ExecStart=/usr/local/bin/nexuslb start --config /etc/nexuslb/nexuslb.yaml
ExecReload=/usr/local/bin/nexuslb reload
Restart=always
RestartSec=2s
LimitNOFILE=1048576

# Security Sandboxing
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_BIND_SERVICE
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target`,
          },
        ],
      },
      {
        id: "benchmarking",
        title: "8. Benchmark Showdown Data",
        icon: <Zap size={17} />,
        summary: "Empirical benchmarking methodology, reproducible wrk commands, and head-to-head comparison charts.",
        subsections: [
          {
            id: "methodology",
            title: "Reproducible Benchmarking Methodology",
            desc: "Tested on identical hardware (Apple Silicon 8-Core, loopback mock HTTP backends, C=100 concurrency). Throughput measured over 60-second sustain phases.",
            codeLang: "bash",
            code: `# Run high-concurrency wrk benchmark with HDR histogram tracking
wrk -t8 -c100 -d60s --latency http://127.0.0.1:8080/

# Test with pipelining bombardier
bombardier -c 250 -n 1000000 http://127.0.0.1:8080/`,
            table: {
              headers: ["Competitor", "Throughput (C=50)", "P99 Latency", "Memory (RSS)"],
              rows: [
                ["NexusLB v0.0.1", "118,872 req/s", "0.41 ms", "7.9 MB"],
                ["NGINX v1.31 (Prod)", "109,160 req/s", "1.32 ms", "256.6 MB"],
                ["UltraBalancer v3", "83,152 req/s", "2.40 ms", "39.5 MB"],
                ["HAProxy v2.8", "111,200 req/s", "0.98 ms", "24.1 MB"],
              ],
            },
          },
        ],
      },
    ],
    []
  );

  // Active module object
  const currentModule = docModules.find((m) => m.id === activeCategory) || docModules[0];

  // Filtering for search query
  const filteredSubsections = useMemo(() => {
    if (!searchQuery.trim()) return currentModule.subsections;
    const q = searchQuery.toLowerCase();
    return currentModule.subsections.filter(
      (sub) =>
        sub.title.toLowerCase().includes(q) ||
        (sub.desc && sub.desc.toLowerCase().includes(q)) ||
        (sub.code && sub.code.toLowerCase().includes(q))
    );
  }, [currentModule, searchQuery]);

  // Next / Previous navigation calculation
  const currentIndex = docModules.findIndex((m) => m.id === activeCategory);
  const prevModule = currentIndex > 0 ? docModules[currentIndex - 1] : null;
  const nextModule = currentIndex < docModules.length - 1 ? docModules[currentIndex + 1] : null;

  return (
    <div id="docs" style={{ maxWidth: "1280px", margin: "0 auto", padding: "32px 20px" }}>
      {/* Top Banner / Search bar */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          flexWrap: "wrap",
          gap: "16px",
          marginBottom: "32px",
          padding: "16px 24px",
          borderRadius: "16px",
          background: "rgba(22, 24, 34, 0.75)",
          backdropFilter: "blur(24px) saturate(180%)",
          border: "1px solid rgba(255, 255, 255, 0.1)",
          boxShadow: "0 8px 32px rgba(0, 0, 0, 0.4)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "10px", fontSize: "14px" }}>
          <span style={{ color: "var(--text-muted)" }}>Documentation</span>
          <span style={{ color: "rgba(255, 255, 255, 0.2)" }}>/</span>
          <span style={{ color: "#ffffff", fontWeight: 600 }}>{currentModule.title}</span>
        </div>

        {/* Global Search Input */}
        <div style={{ position: "relative", minWidth: "280px" }}>
          <Search
            size={16}
            style={{
              position: "absolute",
              left: "14px",
              top: "50%",
              transform: "translateY(-50%)",
              color: "var(--text-muted)",
            }}
          />
          <input
            type="text"
            placeholder="Search guides, code, formulas..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{
              width: "100%",
              padding: "9px 14px 9px 38px",
              borderRadius: "100px",
              background: "rgba(0, 0, 0, 0.4)",
              border: "1px solid rgba(255, 255, 255, 0.12)",
              color: "#ffffff",
              fontSize: "13px",
              outline: "none",
              transition: "all 0.2s ease",
            }}
            onFocus={(e) => (e.target.style.borderColor = "#2997ff")}
            onBlur={(e) => (e.target.style.borderColor = "rgba(255, 255, 255, 0.12)")}
          />
        </div>
      </div>

      {/* Main Multi-Column Split View */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "280px minmax(0, 1fr) 220px",
          gap: "32px",
          alignItems: "start",
        }}
      >
        {/* Left Column: Module Categories Sidebar */}
        <aside
          style={{
            position: "sticky",
            top: "96px",
            display: "flex",
            flexDirection: "column",
            gap: "6px",
            background: "rgba(16, 18, 27, 0.65)",
            backdropFilter: "blur(20px)",
            borderRadius: "18px",
            padding: "16px 12px",
            border: "1px solid rgba(255, 255, 255, 0.08)",
          }}
        >
          <div
            style={{
              fontSize: "11px",
              fontWeight: 700,
              textTransform: "uppercase",
              letterSpacing: "0.08em",
              color: "var(--text-muted)",
              padding: "6px 12px 10px",
              borderBottom: "1px solid var(--border-subtle)",
              marginBottom: "6px",
            }}
          >
            Documentation Modules
          </div>

          {docModules.map((m) => {
            const isActive = m.id === activeCategory;
            return (
              <button
                key={m.id}
                onClick={() => {
                  setActiveCategory(m.id);
                  setActiveSubId(m.subsections[0]?.id || "");
                }}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "10px 14px",
                  borderRadius: "10px",
                  background: isActive ? "rgba(0, 113, 227, 0.15)" : "transparent",
                  border: isActive ? "1px solid rgba(0, 113, 227, 0.35)" : "1px solid transparent",
                  color: isActive ? "#ffffff" : "var(--text-secondary)",
                  fontWeight: isActive ? 600 : 500,
                  fontSize: "13px",
                  cursor: "pointer",
                  textAlign: "left",
                  transition: "all 0.18s cubic-bezier(0.16, 1, 0.3, 1)",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                  <span style={{ color: isActive ? "#2997ff" : "var(--text-muted)" }}>{m.icon}</span>
                  <span>{m.title}</span>
                </div>
                {isActive && <ChevronRight size={14} color="#2997ff" />}
              </button>
            );
          })}
        </aside>

        {/* Center Column: Detailed Documentation Content */}
        <div style={{ display: "flex", flexDirection: "column", gap: "40px", minWidth: 0 }}>
          {/* Module Header */}
          <div
            style={{
              paddingBottom: "24px",
              borderBottom: "1px solid var(--border-subtle)",
            }}
          >
            <div
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: "8px",
                padding: "3px 10px",
                borderRadius: "100px",
                background: "rgba(0, 113, 227, 0.12)",
                border: "1px solid rgba(0, 113, 227, 0.25)",
                color: "#2997ff",
                fontSize: "12px",
                fontWeight: 600,
                marginBottom: "12px",
              }}
            >
              Module {currentIndex + 1} of {docModules.length}
            </div>
            <h1
              style={{
                fontSize: "clamp(2rem, 3.5vw, 2.6rem)",
                fontWeight: 800,
                letterSpacing: "-0.03em",
                color: "#ffffff",
                lineHeight: 1.15,
                margin: "0 0 12px",
              }}
            >
              {currentModule.title}
            </h1>
            <p style={{ fontSize: "15px", color: "var(--text-secondary)", lineHeight: 1.6, margin: 0 }}>
              {currentModule.summary}
            </p>
          </div>

          {/* Subsections Content */}
          {filteredSubsections.length === 0 ? (
            <div
              style={{
                padding: "48px 24px",
                textAlign: "center",
                borderRadius: "16px",
                background: "rgba(255, 255, 255, 0.02)",
                border: "1px dashed rgba(255, 255, 255, 0.1)",
                color: "var(--text-muted)",
              }}
            >
              No matching sections found for &quot;{searchQuery}&quot;. Try broadening your keywords.
            </div>
          ) : (
            filteredSubsections.map((sub) => (
              <section
                key={sub.id}
                id={sub.id}
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: "16px",
                  scrollMarginTop: "120px",
                }}
              >
                <h2
                  style={{
                    fontSize: "22px",
                    fontWeight: 700,
                    letterSpacing: "-0.02em",
                    color: "#ffffff",
                    margin: 0,
                    display: "flex",
                    alignItems: "center",
                    gap: "10px",
                  }}
                >
                  <span
                    style={{
                      width: "8px",
                      height: "8px",
                      borderRadius: "50%",
                      background: "#2997ff",
                      display: "inline-block",
                    }}
                  />
                  {sub.title}
                </h2>

                {sub.desc && (
                  <p style={{ fontSize: "14.5px", color: "var(--text-secondary)", lineHeight: 1.65, margin: 0 }}>
                    {sub.desc}
                  </p>
                )}

                {/* Bullet List */}
                {sub.bullets && (
                  <ul
                    style={{
                      margin: "4px 0 0",
                      paddingLeft: "20px",
                      display: "flex",
                      flexDirection: "column",
                      gap: "8px",
                      fontSize: "14px",
                      color: "var(--text-secondary)",
                      lineHeight: 1.6,
                    }}
                  >
                    {sub.bullets.map((b, bIdx) => (
                      <li key={bIdx}>{b}</li>
                    ))}
                  </ul>
                )}

                {/* Callout Alert */}
                {sub.callout && (
                  <div
                    style={{
                      padding: "16px 20px",
                      borderRadius: "12px",
                      background:
                        sub.callout.type === "important"
                          ? "rgba(255, 159, 10, 0.1)"
                          : sub.callout.type === "tip"
                          ? "rgba(48, 209, 88, 0.1)"
                          : "rgba(0, 113, 227, 0.1)",
                      border:
                        sub.callout.type === "important"
                          ? "1px solid rgba(255, 159, 10, 0.25)"
                          : sub.callout.type === "tip"
                          ? "1px solid rgba(48, 209, 88, 0.25)"
                          : "1px solid rgba(0, 113, 227, 0.25)",
                      display: "flex",
                      alignItems: "flex-start",
                      gap: "12px",
                      fontSize: "13.5px",
                      lineHeight: 1.6,
                      color: "var(--text-primary)",
                    }}
                  >
                    <Info
                      size={18}
                      color={
                        sub.callout.type === "important"
                          ? "#ff9f0a"
                          : sub.callout.type === "tip"
                          ? "#30d158"
                          : "#2997ff"
                      }
                      style={{ flexShrink: 0, marginTop: "2px" }}
                    />
                    <div>{sub.callout.text}</div>
                  </div>
                )}

                {/* Code Block with Copy Button */}
                {sub.code && (
                  <div
                    style={{
                      position: "relative",
                      borderRadius: "14px",
                      background: "rgba(10, 11, 16, 0.9)",
                      border: "1px solid rgba(255, 255, 255, 0.12)",
                      overflow: "hidden",
                      marginTop: "6px",
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                        padding: "10px 16px",
                        background: "rgba(255, 255, 255, 0.03)",
                        borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
                        fontSize: "12px",
                        color: "var(--text-muted)",
                      }}
                    >
                      <span style={{ fontFamily: "var(--font-mono)", textTransform: "uppercase" }}>
                        {sub.codeLang || "shell"}
                      </span>
                      <button
                        onClick={() => copyCode(sub.code!, sub.id)}
                        style={{
                          display: "inline-flex",
                          alignItems: "center",
                          gap: "6px",
                          background: "rgba(255, 255, 255, 0.06)",
                          border: "1px solid rgba(255, 255, 255, 0.1)",
                          borderRadius: "6px",
                          padding: "4px 8px",
                          fontSize: "11px",
                          color: copiedId === sub.id ? "#30d158" : "var(--text-secondary)",
                          cursor: "pointer",
                          transition: "all 0.15s ease",
                        }}
                      >
                        {copiedId === sub.id ? (
                          <>
                            <Check size={12} /> Copied!
                          </>
                        ) : (
                          <>
                            <Copy size={12} /> Copy Code
                          </>
                        )}
                      </button>
                    </div>
                    <pre
                      style={{
                        margin: 0,
                        padding: "18px",
                        overflowX: "auto",
                        fontSize: "13px",
                        lineHeight: 1.6,
                        fontFamily: "var(--font-mono)",
                        color: "#f5f5f7",
                      }}
                    >
                      <code>{sub.code}</code>
                    </pre>
                  </div>
                )}

                {/* Table Component */}
                {sub.table && (
                  <div
                    style={{
                      borderRadius: "14px",
                      overflowX: "auto",
                      border: "1px solid var(--border-subtle)",
                      marginTop: "8px",
                    }}
                  >
                    <table
                      style={{
                        width: "100%",
                        borderCollapse: "collapse",
                        fontSize: "13px",
                        textAlign: "left",
                      }}
                    >
                      <thead>
                        <tr style={{ background: "rgba(255, 255, 255, 0.04)" }}>
                          {sub.table.headers.map((h, hIdx) => (
                            <th
                              key={hIdx}
                              style={{
                                padding: "12px 16px",
                                borderBottom: "1px solid var(--border-subtle)",
                                color: "#ffffff",
                                fontWeight: 600,
                              }}
                            >
                              {h}
                            </th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {sub.table.rows.map((r, rIdx) => (
                          <tr
                            key={rIdx}
                            style={{
                              borderBottom:
                                rIdx === sub.table!.rows.length - 1 ? "none" : "1px solid var(--border-subtle)",
                            }}
                          >
                            {r.map((cell, cIdx) => (
                              <td
                                key={cIdx}
                                style={{
                                  padding: "12px 16px",
                                  color: cIdx === 0 ? "#ffffff" : "var(--text-secondary)",
                                  fontWeight: cIdx === 0 ? 600 : 400,
                                }}
                              >
                                {cell}
                              </td>
                            ))}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </section>
            ))
          )}

          {/* Previous / Next Article Navigation Footer */}
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              gap: "16px",
              paddingTop: "32px",
              borderTop: "1px solid var(--border-subtle)",
              marginTop: "24px",
              flexWrap: "wrap",
            }}
          >
            {prevModule ? (
              <button
                onClick={() => {
                  setActiveCategory(prevModule.id);
                  setActiveSubId(prevModule.subsections[0]?.id || "");
                  window.scrollTo({ top: 300, behavior: "smooth" });
                }}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "10px",
                  padding: "12px 18px",
                  borderRadius: "12px",
                  background: "rgba(255, 255, 255, 0.04)",
                  border: "1px solid rgba(255, 255, 255, 0.1)",
                  color: "#ffffff",
                  fontSize: "13px",
                  cursor: "pointer",
                  transition: "all 0.2s ease",
                }}
              >
                <ArrowLeft size={16} />
                <div style={{ textAlign: "left" }}>
                  <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>Previous Topic</div>
                  <div style={{ fontWeight: 600 }}>{prevModule.title}</div>
                </div>
              </button>
            ) : (
              <div />
            )}

            {nextModule ? (
              <button
                onClick={() => {
                  setActiveCategory(nextModule.id);
                  setActiveSubId(nextModule.subsections[0]?.id || "");
                  window.scrollTo({ top: 300, behavior: "smooth" });
                }}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "10px",
                  padding: "12px 18px",
                  borderRadius: "12px",
                  background: "rgba(0, 113, 227, 0.15)",
                  border: "1px solid rgba(0, 113, 227, 0.35)",
                  color: "#ffffff",
                  fontSize: "13px",
                  cursor: "pointer",
                  transition: "all 0.2s ease",
                }}
              >
                <div style={{ textAlign: "right" }}>
                  <div style={{ fontSize: "11px", color: "#2997ff" }}>Next Topic</div>
                  <div style={{ fontWeight: 600 }}>{nextModule.title}</div>
                </div>
                <ArrowRight size={16} color="#2997ff" />
              </button>
            ) : (
              <div />
            )}
          </div>
        </div>

        {/* Right Column: "On This Page" Sticky Table of Contents */}
        <aside
          style={{
            position: "sticky",
            top: "96px",
            display: "flex",
            flexDirection: "column",
            gap: "8px",
            padding: "16px 14px",
            borderRadius: "16px",
            background: "rgba(16, 18, 27, 0.45)",
            border: "1px solid rgba(255, 255, 255, 0.06)",
          }}
        >
          <div
            style={{
              fontSize: "11px",
              fontWeight: 700,
              textTransform: "uppercase",
              letterSpacing: "0.08em",
              color: "var(--text-muted)",
              marginBottom: "4px",
            }}
          >
            On This Page
          </div>

          {currentModule.subsections.map((sub) => (
            <a
              key={sub.id}
              href={`#${sub.id}`}
              onClick={() => setActiveSubId(sub.id)}
              style={{
                fontSize: "12.5px",
                color: activeSubId === sub.id ? "#2997ff" : "var(--text-secondary)",
                textDecoration: "none",
                lineHeight: 1.4,
                padding: "6px 8px",
                borderRadius: "6px",
                transition: "all 0.15s ease",
                background: activeSubId === sub.id ? "rgba(0, 113, 227, 0.08)" : "transparent",
              }}
            >
              {sub.title}
            </a>
          ))}

          <div
            style={{
              marginTop: "16px",
              paddingTop: "16px",
              borderTop: "1px solid var(--border-subtle)",
              display: "flex",
              flexDirection: "column",
              gap: "8px",
            }}
          >
            <a
              href="https://github.com/Aazann/NexusLB"
              target="_blank"
              rel="noreferrer"
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: "6px",
                fontSize: "12px",
                color: "var(--text-muted)",
                textDecoration: "none",
              }}
            >
              GitHub Source <ExternalLink size={11} />
            </a>
            <a
              href="/privacy"
              style={{
                fontSize: "12px",
                color: "var(--text-muted)",
                textDecoration: "none",
              }}
            >
              Privacy Policy
            </a>
          </div>
        </aside>
      </div>
    </div>
  );
}
