"use client";

import { Zap, Cpu, RefreshCw, HeartPulse, Layers, Database, Lock, Eye, Route } from "lucide-react";

export function Features() {
  const pillars = [
    {
      icon: <Zap size={22} color="#f5f5f7" />,
      title: "SIMD Vectorized Header Parsing",
      desc: "Uses 128-bit vector instructions (SSE/AVX/NEON) to parse HTTP headers in bulk, replacing slow character-by-character C state machines.",
      tag: "5x–10x Faster",
    },
    {
      icon: <Cpu size={22} color="#f5f5f7" />,
      title: "Lock-Free Buffer & Connection Pools",
      desc: "Pre-allocated ArrayQueue pools eliminate malloc/free syscall overhead on the hot path, achieving true zero-allocation request forwarding.",
      tag: "Zero Allocation",
    },
    {
      icon: <RefreshCw size={22} color="#f5f5f7" />,
      title: "Atomic Zero-Downtime Hot Reload",
      desc: "Reconfigure routes, pools, and SSL certs via POST /reload or SIGHUP in under 1 µs using ArcSwap with zero dropped in-flight requests.",
      tag: "< 1 µs Swap",
    },
    {
      icon: <HeartPulse size={22} color="#f5f5f7" />,
      title: "Active Health & Circuit Breakers",
      desc: "Native background HTTP/TCP health probing and 3-state circuit breakers isolate failing backends without requiring paid enterprise licenses.",
      tag: "Built-In",
    },
    {
      icon: <Layers size={22} color="#f5f5f7" />,
      title: "Pluggable Multi-Engine Architecture",
      desc: "Seamlessly switch between high-concurrency Tokio, Linux kernel io_uring, and bare-metal kernel-bypass AF_XDP for maximum throughput.",
      tag: "Cross-Platform",
    },
    {
      icon: <Database size={22} color="#f5f5f7" />,
      title: "RFC 7234 In-Memory Cache",
      desc: "High-speed in-memory LRU cache with conditional ETag revalidation serving cached responses with 304 Not Modified in microseconds.",
      tag: "Sub-ms HIT",
    },
    {
      icon: <Lock size={22} color="#f5f5f7" />,
      title: "TLS Termination & Dynamic SNI",
      desc: "Hardware-accelerated TLS 1.3 / HTTP/2 termination using PrefixedStream, supporting wildcard domains and dynamic certificate reloads.",
      tag: "Zero Packet Loss",
    },
    {
      icon: <Eye size={22} color="#f5f5f7" />,
      title: "Real-Time Terminal Dashboard",
      desc: "Interactive terminal TUI (nexuslb top) and Prometheus exporter for zero-overhead, sub-millisecond observability.",
      tag: "10Hz Live Stats",
    },
    {
      icon: <Route size={22} color="#f5f5f7" />,
      title: "Extensible Filter Pipeline",
      desc: "JWT authentication, header injection, and distributed tracing (W3C traceparent OpenTelemetry) evaluated before backend dispatch.",
      tag: "Cloud Native",
    },
  ];

  return (
    <section id="architecture" className="section" style={{ position: "relative" }}>
      <div className="container">
        <div className="section-label">Engineering Foundations</div>
        <h2 className="section-title">Zero Allocation. Lock-Free. Safe Rust.</h2>
        <p className="section-desc">
          Every microsecond matters. NexusLB combines low-level OS capabilities with modern Rust concurrency abstractions to deliver unmatched performance.
        </p>

        {/* Bento Grid with Generous Padding */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 280px), 1fr))",
            gap: "24px",
          }}
        >
          {pillars.map((p, idx) => (
            <div
              key={idx}
              className="apple-card"
              style={{
                display: "flex",
                flexDirection: "column",
                justifyContent: "space-between",
                padding: "32px 28px",
              }}
            >
              <div>
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "20px" }}>
                  <div
                    style={{
                      width: "44px",
                      height: "44px",
                      borderRadius: "12px",
                      backgroundColor: "rgba(255, 255, 255, 0.06)",
                      border: "1px solid rgba(255, 255, 255, 0.1)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                    }}
                  >
                    {p.icon}
                  </div>
                  <span
                    style={{
                      fontSize: "11.5px",
                      fontWeight: 500,
                      fontFamily: "var(--font-mono)",
                      padding: "3px 9px",
                      borderRadius: "9999px",
                      backgroundColor: "rgba(255, 255, 255, 0.05)",
                      color: "var(--text-secondary)",
                      border: "1px solid rgba(255, 255, 255, 0.08)",
                    }}
                  >
                    {p.tag}
                  </span>
                </div>

                <h3 style={{ fontSize: "18px", fontWeight: 700, color: "#ffffff", marginBottom: "10px", letterSpacing: "-0.015em" }}>
                  {p.title}
                </h3>
                <p style={{ fontSize: "14px", color: "var(--text-secondary)", lineHeight: 1.65 }}>
                  {p.desc}
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
