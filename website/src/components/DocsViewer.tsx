"use client";

import { useState } from "react";
import { BookOpen, Terminal, Settings, Cpu, Shield, BarChart3, Search, Copy, Check, ArrowRight } from "lucide-react";

export function DocsViewer() {
  const [activeSection, setActiveSection] = useState<string>("getting-started");
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  const copyCode = (code: string, id: string) => {
    navigator.clipboard.writeText(code);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const sections = [
    { id: "getting-started", title: "Getting Started", icon: <Terminal size={16} /> },
    { id: "architecture", title: "Architecture & Hot-Path", icon: <Cpu size={16} /> },
    { id: "configuration", title: "Configuration Reference", icon: <Settings size={16} /> },
    { id: "algorithms", title: "10 Scheduling Algorithms", icon: <BarChart3 size={16} /> },
    { id: "production", title: "Production & Kernel Tuning", icon: <Shield size={16} /> },
    { id: "benchmarks", title: "Benchmark Showdown Data", icon: <BookOpen size={16} /> },
  ];

  return (
    <div style={{ maxWidth: "1140px", margin: "0 auto", padding: "40px 0" }}>
      {/* Search & Breadcrumb Bar */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          flexWrap: "wrap",
          gap: "16px",
          marginBottom: "32px",
          padding: "16px 24px",
          borderRadius: "var(--radius-md)",
          backgroundColor: "rgba(255, 255, 255, 0.04)",
          border: "1px solid rgba(255, 255, 255, 0.08)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "14px", color: "var(--text-tertiary)" }}>
          <span>Documentation</span>
          <span>/</span>
          <span style={{ color: "#ffffff", fontWeight: 600 }}>
            {sections.find((s) => s.id === activeSection)?.title}
          </span>
        </div>

        <div style={{ position: "relative", minWidth: "260px" }}>
          <Search size={15} color="var(--text-tertiary)" style={{ position: "absolute", left: "12px", top: "11px" }} />
          <input
            type="text"
            placeholder="Search documentation..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{
              width: "100%",
              padding: "8px 12px 8px 36px",
              borderRadius: "var(--radius-pill)",
              backgroundColor: "rgba(0, 0, 0, 0.4)",
              border: "1px solid rgba(255, 255, 255, 0.12)",
              color: "#ffffff",
              fontSize: "13px",
              outline: "none",
            }}
          />
        </div>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "260px 1fr",
          gap: "36px",
          alignItems: "start",
        }}
        className="docs-grid"
      >
        {/* Apple Sidebar Navigation */}
        <aside
          style={{
            position: "sticky",
            top: "90px",
            backgroundColor: "rgba(18, 20, 28, 0.65)",
            backdropFilter: "blur(20px)",
            borderRadius: "var(--radius-md)",
            border: "1px solid rgba(255, 255, 255, 0.08)",
            padding: "12px",
            display: "flex",
            flexDirection: "column",
            gap: "4px",
          }}
        >
          <div style={{ fontSize: "11px", fontWeight: 700, textTransform: "uppercase", color: "var(--text-tertiary)", padding: "8px 12px 4px", letterSpacing: "0.06em" }}>
            Documentation Guides
          </div>
          {sections.map((s) => (
            <button
              key={s.id}
              onClick={() => setActiveSection(s.id)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "10px",
                padding: "10px 14px",
                borderRadius: "var(--radius-sm)",
                border: "none",
                backgroundColor: activeSection === s.id ? "rgba(0, 113, 227, 0.25)" : "transparent",
                color: activeSection === s.id ? "#ffffff" : "var(--text-secondary)",
                fontWeight: activeSection === s.id ? 600 : 400,
                fontSize: "13.5px",
                cursor: "pointer",
                textAlign: "left",
                transition: "all 0.2s var(--spring-snappy)",
              }}
            >
              <span style={{ color: activeSection === s.id ? "var(--accent-cyan)" : "inherit" }}>{s.icon}</span>
              <span>{s.title}</span>
            </button>
          ))}
        </aside>

        {/* Content Body */}
        <main
          className="apple-card"
          style={{
            padding: "36px 40px",
            backgroundColor: "rgba(14, 16, 23, 0.75)",
            minHeight: "650px",
          }}
        >
          {activeSection === "getting-started" && (
            <div>
              <h2 style={{ fontSize: "28px", fontWeight: 700, color: "#ffffff", marginBottom: "8px" }}>
                Getting Started with NexusLB
              </h2>
              <p style={{ color: "var(--text-secondary)", fontSize: "15px", marginBottom: "24px" }}>
                NexusLB is a high-performance Layer 4 and Layer 7 load balancer written in safe Rust. Follow this quickstart guide to compile, configure, and launch your first cluster in under 60 seconds.
              </p>

              <h3 style={{ fontSize: "18px", fontWeight: 600, color: "#ffffff", marginTop: "28px", marginBottom: "12px" }}>
                1. Prerequisites & Compilation
              </h3>
              <p style={{ color: "var(--text-secondary)", fontSize: "14px", marginBottom: "12px" }}>
                Ensure Rust 1.80+ is installed on your system. NexusLB is compiled with Fat LTO and single codegen units for peak optimization:
              </p>

              <div style={{ position: "relative", marginBottom: "20px" }}>
                <pre style={{ backgroundColor: "#090c15", padding: "16px", borderRadius: "var(--radius-sm)", overflowX: "auto", border: "1px solid rgba(255, 255, 255, 0.08)" }}>
                  <code style={{ color: "#00f0ff" }}>
{`git clone https://github.com/nexuslb/nexuslb.git
cd nexuslb
cargo build --release`}
                  </code>
                </pre>
                <button
                  onClick={() => copyCode("git clone https://github.com/nexuslb/nexuslb.git\ncd nexuslb\ncargo build --release", "build-code")}
                  style={{ position: "absolute", right: "12px", top: "12px", background: "none", border: "none", color: "var(--text-tertiary)", cursor: "pointer" }}
                >
                  {copiedId === "build-code" ? <Check size={14} color="#30d158" /> : <Copy size={14} />}
                </button>
              </div>

              <h3 style={{ fontSize: "18px", fontWeight: 600, color: "#ffffff", marginTop: "28px", marginBottom: "12px" }}>
                2. CLI Commands Reference
              </h3>
              <div style={{ overflowX: "auto", marginBottom: "24px" }}>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "13.5px", textAlign: "left" }}>
                  <thead>
                    <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.12)", color: "var(--text-tertiary)" }}>
                      <th style={{ padding: "10px" }}>Command</th>
                      <th style={{ padding: "10px" }}>Description</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                      <td style={{ padding: "10px", fontFamily: "var(--font-mono)", color: "var(--accent-cyan)" }}>nexuslb start -c nexuslb.yaml</td>
                      <td style={{ padding: "10px", color: "var(--text-secondary)" }}>Starts the dataplane engine with workers and listener sockets.</td>
                    </tr>
                    <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                      <td style={{ padding: "10px", fontFamily: "var(--font-mono)", color: "var(--accent-cyan)" }}>nexuslb check -c nexuslb.yaml</td>
                      <td style={{ padding: "10px", color: "var(--text-secondary)" }}>Validates configuration syntax and backends without starting.</td>
                    </tr>
                    <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                      <td style={{ padding: "10px", fontFamily: "var(--font-mono)", color: "var(--accent-cyan)" }}>nexuslb reload -a 127.0.0.1:9091</td>
                      <td style={{ padding: "10px", color: "var(--text-secondary)" }}>Signals running instance to atomically reload configuration via ArcSwap.</td>
                    </tr>
                    <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                      <td style={{ padding: "10px", fontFamily: "var(--font-mono)", color: "var(--accent-cyan)" }}>nexuslb top</td>
                      <td style={{ padding: "10px", color: "var(--text-secondary)" }}>Launches interactive real-time terminal TUI dashboard.</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {activeSection === "architecture" && (
            <div>
              <h2 style={{ fontSize: "28px", fontWeight: 700, color: "#ffffff", marginBottom: "8px" }}>
                Architecture & Hot-Path Design
              </h2>
              <p style={{ color: "var(--text-secondary)", fontSize: "15px", marginBottom: "24px" }}>
                NexusLB eliminates hot-path memory allocation, lock contention, and cache bouncing across CPU cores.
              </p>

              <h3 style={{ fontSize: "18px", fontWeight: 600, color: "#ffffff", marginTop: "24px", marginBottom: "8px" }}>
                1. Lock-Free Buffer Pooling
              </h3>
              <p style={{ color: "var(--text-secondary)", fontSize: "14px", lineHeight: 1.6, marginBottom: "16px" }}>
                Incoming TCP streams acquire pre-allocated buffers from an ArrayQueue-backed pool. When a request finishes, the buffer returns to the pool automatically via RAII Drop semantics. Dynamic malloc and free calls are 100% removed from the request forwarding path.
              </p>

              <h3 style={{ fontSize: "18px", fontWeight: 600, color: "#ffffff", marginTop: "24px", marginBottom: "8px" }}>
                2. SIMD Vectorized Parsing
              </h3>
              <p style={{ color: "var(--text-secondary)", fontSize: "14px", lineHeight: 1.6, marginBottom: "16px" }}>
                Leveraging httparse with SSE4.2, AVX2, and ARM Neon instructions, NexusLB scans and parses HTTP/1.1 headers up to 10x faster than traditional byte-by-byte C state machines used in legacy reverse proxies.
              </p>

              <h3 style={{ fontSize: "18px", fontWeight: 600, color: "#ffffff", marginTop: "24px", marginBottom: "8px" }}>
                3. Atomic ArcSwap State Swapping
              </h3>
              <p style={{ color: "var(--text-secondary)", fontSize: "14px", lineHeight: 1.6, marginBottom: "16px" }}>
                DataplaneState is stored inside an ArcSwap pointer. When a configuration reload is triggered via SIGHUP or POST /reload, the new routing graph is compiled in isolation and swapped into place in less than 1 microsecond without taking locks or stalling workers.
              </p>
            </div>
          )}

          {activeSection === "configuration" && (
            <div>
              <h2 style={{ fontSize: "28px", fontWeight: 700, color: "#ffffff", marginBottom: "8px" }}>
                Configuration Reference
              </h2>
              <p style={{ color: "var(--text-secondary)", fontSize: "15px", marginBottom: "24px" }}>
                Full specification of all YAML sections supported by NexusLB.
              </p>

              <div style={{ position: "relative", marginBottom: "20px" }}>
                <pre style={{ backgroundColor: "#090c15", padding: "18px", borderRadius: "var(--radius-sm)", overflowX: "auto", border: "1px solid rgba(255, 255, 255, 0.08)", fontSize: "12.5px", lineHeight: 1.65 }}>
                  <code style={{ color: "#9cdcfe" }}>
{`server:
  listen:
    - "0.0.0.0:80"
    - "0.0.0.0:443"
  workers: "auto"        # "auto" or number of CPU threads
  engine: "tokio"        # "tokio", "io-uring" (Linux), or "xdp"
  reuse_port: true       # SO_REUSEPORT socket kernel load distribution
  tcp_nodelay: true

load_balancer:
  algorithm: "adaptive"  # Options: adaptive, power_of_two_choices, round_robin, etc.
  default_pool: "web"

backends:
  - name: "app-1"
    address: "10.0.1.10:8080"
    weight: 100
    protocol: "http1"
    pool: "web"
    max_connections: 5000

health_check:
  enabled: true
  interval: "5s"
  timeout: "2s"
  healthy_threshold: 2
  unhealthy_threshold: 3
  http_path: "/healthz"
  expected_status: 200

tls:
  enabled: true
  cert_path: "/etc/nexuslb/certs/fullchain.pem"
  key_path: "/etc/nexuslb/certs/privkey.pem"
  redirect_http_to_https: true

rate_limit:
  enabled: true
  global_rps: 120000
  client_rps: 1500

admin:
  enabled: true
  address: "127.0.0.1:9091"`}
                  </code>
                </pre>
              </div>
            </div>
          )}

          {activeSection === "algorithms" && (
            <div>
              <h2 style={{ fontSize: "28px", fontWeight: 700, color: "#ffffff", marginBottom: "8px" }}>
                10 Built-In Scheduling Algorithms
              </h2>
              <p style={{ color: "var(--text-secondary)", fontSize: "15px", marginBottom: "24px" }}>
                NexusLB implements 10 mathematical load balancing algorithms designed for diverse distributed system constraints.
              </p>

              <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
                <div style={{ padding: "16px", borderRadius: "var(--radius-sm)", backgroundColor: "rgba(255, 255, 255, 0.03)", border: "1px solid rgba(255, 255, 255, 0.08)" }}>
                  <div style={{ fontWeight: 600, color: "var(--accent-cyan)", marginBottom: "4px" }}>1. Adaptive Scoring (Default)</div>
                  <p style={{ fontSize: "13.5px", color: "var(--text-secondary)", lineHeight: 1.5 }}>
                    Combines Exponentially Weighted Moving Average (EWMA) latency, active connection ratios, and historical error counts into a composite penalty score: Score = EWMA * (1 + Conns) * (1 + Errors / 10). Automatically steers traffic away from nodes experiencing background GC pauses.
                  </p>
                </div>

                <div style={{ padding: "16px", borderRadius: "var(--radius-sm)", backgroundColor: "rgba(255, 255, 255, 0.03)", border: "1px solid rgba(255, 255, 255, 0.08)" }}>
                  <div style={{ fontWeight: 600, color: "var(--accent-emerald)", marginBottom: "4px" }}>2. Power of Two Choices (P2C)</div>
                  <p style={{ fontSize: "13.5px", color: "var(--text-secondary)", lineHeight: 1.5 }}>
                    Selects two candidate backends at random and picks the one with lower active queue pressure. Eliminates the herd behavior of Least Connections with guaranteed O(1) constant time complexity.
                  </p>
                </div>

                <div style={{ padding: "16px", borderRadius: "var(--radius-sm)", backgroundColor: "rgba(255, 255, 255, 0.03)", border: "1px solid rgba(255, 255, 255, 0.08)" }}>
                  <div style={{ fontWeight: 600, color: "var(--accent-purple)", marginBottom: "4px" }}>3. Consistent Hashing (Ketama Ring)</div>
                  <p style={{ fontSize: "13.5px", color: "var(--text-secondary)", lineHeight: 1.5 }}>
                    160 virtual nodes per replica mapped onto a 32-bit hash ring. Ensures that when nodes are added or removed, only 1/N keys are relocated, preventing cache invalidation storms on Redis/Memcached tiers.
                  </p>
                </div>

                <div style={{ padding: "16px", borderRadius: "var(--radius-sm)", backgroundColor: "rgba(255, 255, 255, 0.03)", border: "1px solid rgba(255, 255, 255, 0.08)" }}>
                  <div style={{ fontWeight: 600, color: "var(--accent-blue)", marginBottom: "4px" }}>4. Peak EWMA, Least Connections, IP Hash & Weighted Round Robin</div>
                  <p style={{ fontSize: "13.5px", color: "var(--text-secondary)", lineHeight: 1.5 }}>
                    Specialized schedulers for gRPC microservice calls, long-lived WebSockets, client IP sticky affinity, and heterogeneous hardware nodes.
                  </p>
                </div>
              </div>
            </div>
          )}

          {activeSection === "production" && (
            <div>
              <h2 style={{ fontSize: "28px", fontWeight: 700, color: "#ffffff", marginBottom: "8px" }}>
                Production Operations & Kernel Tuning
              </h2>
              <p style={{ color: "var(--text-secondary)", fontSize: "15px", marginBottom: "24px" }}>
                To sustain 100,000+ requests per second per node without socket starvation or packet drops, apply the following Linux sysctl parameters:
              </p>

              <pre style={{ backgroundColor: "#090c15", padding: "16px", borderRadius: "var(--radius-sm)", overflowX: "auto", border: "1px solid rgba(255, 255, 255, 0.08)", fontSize: "12.5px" }}>
                <code style={{ color: "#f5a623" }}>
{`# /etc/sysctl.d/99-nexuslb.conf
fs.file-max = 2097152
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535
net.ipv4.tcp_tw_reuse = 1
net.ipv4.ip_local_port_range = 1024 65535
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.core.netdev_max_backlog = 100000`}
                </code>
              </pre>
            </div>
          )}

          {activeSection === "benchmarks" && (
            <div>
              <h2 style={{ fontSize: "28px", fontWeight: 700, color: "#ffffff", marginBottom: "8px" }}>
                Empirical Benchmark Showdown Results
              </h2>
              <p style={{ color: "var(--text-secondary)", fontSize: "15px", marginBottom: "24px" }}>
                Audited performance metrics gathered on identical 8-core Apple Silicon hardware against identical loopback mock servers:
              </p>

              <div style={{ overflowX: "auto" }}>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "13.5px", textAlign: "left" }}>
                  <thead>
                    <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.12)", color: "var(--text-tertiary)" }}>
                      <th style={{ padding: "10px" }}>Metric</th>
                      <th style={{ padding: "10px" }}>NGINX Production</th>
                      <th style={{ padding: "10px" }}>UltraBalancer</th>
                      <th style={{ padding: "10px", color: "var(--accent-cyan)" }}>NexusLB v0.1.0</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                      <td style={{ padding: "10px", fontWeight: 600 }}>Throughput (C=50)</td>
                      <td style={{ padding: "10px" }}>109,160 req/s</td>
                      <td style={{ padding: "10px" }}>83,152 req/s</td>
                      <td style={{ padding: "10px", color: "#30d158", fontWeight: 700 }}>118,872 req/s (+43%)</td>
                    </tr>
                    <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                      <td style={{ padding: "10px", fontWeight: 600 }}>Throughput (C=100)</td>
                      <td style={{ padding: "10px" }}>107,363 req/s</td>
                      <td style={{ padding: "10px" }}>78,296 req/s</td>
                      <td style={{ padding: "10px", color: "#30d158", fontWeight: 700 }}>111,470 req/s (+3.8%)</td>
                    </tr>
                    <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                      <td style={{ padding: "10px", fontWeight: 600 }}>Median Latency (P50)</td>
                      <td style={{ padding: "10px" }}>658 µs</td>
                      <td style={{ padding: "10px" }}>1,137 µs</td>
                      <td style={{ padding: "10px", color: "#00f0ff", fontWeight: 700 }}>525 µs (20.2% faster)</td>
                    </tr>
                    <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                      <td style={{ padding: "10px", fontWeight: 600 }}>Peak Memory (RSS)</td>
                      <td style={{ padding: "10px" }}>22.7 MB</td>
                      <td style={{ padding: "10px" }}>36.4 MB</td>
                      <td style={{ padding: "10px", color: "#bf5af2", fontWeight: 700 }}>2.6 MB (88.6% leaner)</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
