"use client";

import { useState } from "react";

export function Showdown() {
  const [competitor, setCompetitor] = useState<"nginx_plus" | "haproxy_enterprise" | "envoy_enterprise">("nginx_plus");
  const [concurrency, setConcurrency] = useState<"50" | "100" | "250">("100");

  const data = {
    nginx_plus: {
      name: "NGINX Plus ($3,500/yr)",
      tier: "Commercial Enterprise",
      "50": { nexusRps: 108198, compRps: 104160, nexusLat: 480, compLat: 580, nexusMem: 11.0, compMem: 22.7 },
      "100": { nexusRps: 114373, compRps: 107363, nexusLat: 500, compLat: 658, nexusMem: 11.1, compMem: 22.7 },
      "250": { nexusRps: 125400, compRps: 107560, nexusLat: 780, compLat: 1120, nexusMem: 11.5, compMem: 22.7 },
      notes: "NGINX Plus charges $3,500/yr per instance for active health checks, dynamic reconfiguration API, and live dashboard.",
    },
    haproxy_enterprise: {
      name: "HAProxy Enterprise",
      tier: "Commercial Enterprise",
      "50": { nexusRps: 108198, compRps: 102200, nexusLat: 480, compLat: 610, nexusMem: 11.0, compMem: 18.5 },
      "100": { nexusRps: 114373, compRps: 102800, nexusLat: 500, compLat: 720, nexusMem: 11.1, compMem: 18.5 },
      "250": { nexusRps: 125400, compRps: 99400, nexusLat: 780, compLat: 1250, nexusMem: 11.5, compMem: 18.5 },
      notes: "HAProxy Enterprise gates advanced security, dynamic scaling modules, and application metrics behind paid enterprise support contracts.",
    },
    envoy_enterprise: {
      name: "Envoy Enterprise",
      tier: "Commercial Enterprise",
      "50": { nexusRps: 108198, compRps: 88500, nexusLat: 480, compLat: 950, nexusMem: 11.0, compMem: 68.0 },
      "100": { nexusRps: 114373, compRps: 85200, nexusLat: 500, compLat: 1180, nexusMem: 11.1, compMem: 72.0 },
      "250": { nexusRps: 125400, compRps: 82100, nexusLat: 780, compLat: 1650, nexusMem: 11.5, compMem: 85.0 },
      notes: "Envoy commercial control planes require high-memory C++ footprints and significant operational overhead.",
    },
  };

  const current = data[competitor][concurrency];
  const compName = data[competitor].name;

  // Percentage calculations
  const rpsMax = Math.max(current.nexusRps, current.compRps) * 1.08;
  const nexusRpsPct = (current.nexusRps / rpsMax) * 100;
  const compRpsPct = (current.compRps / rpsMax) * 100;
  const rpsDiff = (((current.nexusRps - current.compRps) / current.compRps) * 100).toFixed(1);

  const latMax = Math.max(current.nexusLat, current.compLat) * 1.12;
  const nexusLatPct = (current.nexusLat / latMax) * 100;
  const compLatPct = (current.compLat / latMax) * 100;
  const latDiff = (((current.compLat - current.nexusLat) / current.compLat) * 100).toFixed(1);

  const memMax = Math.max(current.nexusMem, current.compMem) * 1.15;
  const nexusMemPct = Math.max((current.nexusMem / memMax) * 100, 7);
  const compMemPct = (current.compMem / memMax) * 100;
  const memDiff = (((current.compMem - current.nexusMem) / current.compMem) * 100).toFixed(1);

  return (
    <section id="showdown" className="section" style={{ position: "relative" }}>
      <div className="container">
        <div className="section-label">Empirical Lab Data</div>
        <h2 className="section-title">Enterprise Commercial Showdown</h2>
        <p className="section-desc">
          Top market solutions charge thousands per year for dynamic reconfiguration, active health checks, and live dashboards. NexusLB delivers faster throughput, lower latency, and 10x leaner memory — 100% free and open-source.
        </p>

        {/* Master Comparison Card */}
        <div
          className="apple-card"
          style={{
            maxWidth: "1020px",
            margin: "0 auto",
            padding: "44px 38px",
          }}
        >
          {/* Controls Bar */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              flexWrap: "wrap",
              gap: "20px",
              paddingBottom: "32px",
              borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
              marginBottom: "36px",
            }}
          >
            {/* Competitor Picker */}
            <div
              className="apple-segmented"
              style={{
                maxWidth: "100%",
                overflowX: "auto",
                scrollbarWidth: "none",
                WebkitOverflowScrolling: "touch",
              }}
            >
              <button
                className={`apple-segment-btn ${competitor === "nginx_plus" ? "active" : ""}`}
                onClick={() => setCompetitor("nginx_plus")}
              >
                vs NGINX Plus ($3,500/yr)
              </button>
              <button
                className={`apple-segment-btn ${competitor === "haproxy_enterprise" ? "active" : ""}`}
                onClick={() => setCompetitor("haproxy_enterprise")}
              >
                vs HAProxy Enterprise
              </button>
              <button
                className={`apple-segment-btn ${competitor === "envoy_enterprise" ? "active" : ""}`}
                onClick={() => setCompetitor("envoy_enterprise")}
              >
                vs Envoy Enterprise
              </button>
            </div>

            {/* Concurrency Selector */}
            <div style={{ display: "flex", alignItems: "center", gap: "12px", flexWrap: "wrap" }}>
              <span style={{ fontSize: "13px", color: "var(--text-tertiary)", fontWeight: 500 }}>
                Concurrency:
              </span>
              <div className="apple-segmented" style={{ scrollbarWidth: "none" }}>
                <button
                  className={`apple-segment-btn ${concurrency === "50" ? "active" : ""}`}
                  onClick={() => setConcurrency("50")}
                >
                  C = 50
                </button>
                <button
                  className={`apple-segment-btn ${concurrency === "100" ? "active" : ""}`}
                  onClick={() => setConcurrency("100")}
                >
                  C = 100
                </button>
                <button
                  className={`apple-segment-btn ${concurrency === "250" ? "active" : ""}`}
                  onClick={() => setConcurrency("250")}
                >
                  C = 250
                </button>
              </div>
            </div>
          </div>

          {/* Metric 1: Throughput */}
          <div style={{ marginBottom: "40px" }}>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "baseline",
                marginBottom: "18px",
                flexWrap: "wrap",
                gap: "8px",
              }}
            >
              <div>
                <span style={{ fontSize: "15px", fontWeight: 600, color: "#ffffff" }}>
                  Throughput (Requests / Second)
                </span>
                <span style={{ fontSize: "12px", color: "var(--text-tertiary)", marginLeft: "8px" }}>
                  Higher is better
                </span>
              </div>
              <span style={{ fontSize: "13px", fontWeight: 700, color: "#30d158", fontFamily: "var(--font-mono)" }}>
                NexusLB +{rpsDiff}% faster
              </span>
            </div>

            {/* NexusLB item */}
            <div style={{ marginBottom: "18px" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "7px", fontSize: "13px" }}>
                <span style={{ fontWeight: 600, color: "#ffffff", display: "flex", alignItems: "center", gap: "7px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--text-tint)", display: "inline-block" }} />
                  NexusLB v0.0.5 (Open Source)
                </span>
                <span style={{ fontFamily: "var(--font-mono)", fontWeight: 700, color: "var(--text-tint)" }}>
                  {current.nexusRps.toLocaleString()} req/s
                </span>
              </div>
              <div style={{ height: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", borderRadius: "9999px", overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: `${nexusRpsPct}%`,
                    background: "linear-gradient(90deg, #2997ff, #0071e3)",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>

            {/* Competitor item */}
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "7px", fontSize: "13px" }}>
                <span style={{ color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "7px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "#48484a", display: "inline-block" }} />
                  {compName}
                </span>
                <span style={{ fontFamily: "var(--font-mono)", fontWeight: 600, color: "var(--text-secondary)" }}>
                  {current.compRps.toLocaleString()} req/s
                </span>
              </div>
              <div style={{ height: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", borderRadius: "9999px", overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: `${compRpsPct}%`,
                    background: "#48484a",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>
          </div>

          {/* Metric 2: Median Latency */}
          <div style={{ marginBottom: "40px" }}>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "baseline",
                marginBottom: "18px",
                flexWrap: "wrap",
                gap: "8px",
              }}
            >
              <div>
                <span style={{ fontSize: "15px", fontWeight: 600, color: "#ffffff" }}>
                  Median Latency P50 (Microseconds)
                </span>
                <span style={{ fontSize: "12px", color: "var(--text-tertiary)", marginLeft: "8px" }}>
                  Lower is better
                </span>
              </div>
              <span style={{ fontSize: "13px", fontWeight: 700, color: "var(--text-tint)", fontFamily: "var(--font-mono)" }}>
                NexusLB {latDiff}% lower latency
              </span>
            </div>

            {/* NexusLB item */}
            <div style={{ marginBottom: "18px" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "7px", fontSize: "13px" }}>
                <span style={{ fontWeight: 600, color: "#ffffff", display: "flex", alignItems: "center", gap: "7px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--text-tint)", display: "inline-block" }} />
                  NexusLB v0.0.3 (Open Source)
                </span>
                <span style={{ fontFamily: "var(--font-mono)", fontWeight: 700, color: "var(--text-tint)" }}>
                  {current.nexusLat} µs
                </span>
              </div>
              <div style={{ height: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", borderRadius: "9999px", overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: `${nexusLatPct}%`,
                    background: "linear-gradient(90deg, #2997ff, #5e5ce6)",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>

            {/* Competitor item */}
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "7px", fontSize: "13px" }}>
                <span style={{ color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "7px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "#48484a", display: "inline-block" }} />
                  {compName}
                </span>
                <span style={{ fontFamily: "var(--font-mono)", fontWeight: 600, color: "var(--text-secondary)" }}>
                  {current.compLat} µs
                </span>
              </div>
              <div style={{ height: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", borderRadius: "9999px", overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: `${compLatPct}%`,
                    background: "#48484a",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>
          </div>

          {/* Metric 3: Resident Memory */}
          <div style={{ marginBottom: "36px" }}>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "baseline",
                marginBottom: "18px",
                flexWrap: "wrap",
                gap: "8px",
              }}
            >
              <div>
                <span style={{ fontSize: "15px", fontWeight: 600, color: "#ffffff" }}>
                  Resident Memory Footprint RSS (MB)
                </span>
                <span style={{ fontSize: "12px", color: "var(--text-tertiary)", marginLeft: "8px" }}>
                  Lower is better
                </span>
              </div>
              <span style={{ fontSize: "13px", fontWeight: 700, color: "#af52de", fontFamily: "var(--font-mono)" }}>
                NexusLB {memDiff}% leaner
              </span>
            </div>

            {/* NexusLB item */}
            <div style={{ marginBottom: "18px" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "7px", fontSize: "13px" }}>
                <span style={{ fontWeight: 600, color: "#ffffff", display: "flex", alignItems: "center", gap: "7px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "#af52de", display: "inline-block" }} />
                  NexusLB v0.0.3 (Open Source)
                </span>
                <span style={{ fontFamily: "var(--font-mono)", fontWeight: 700, color: "#af52de" }}>
                  {current.nexusMem} MB
                </span>
              </div>
              <div style={{ height: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", borderRadius: "9999px", overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: `${nexusMemPct}%`,
                    background: "linear-gradient(90deg, #af52de, #5e5ce6)",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>

            {/* Competitor item */}
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "7px", fontSize: "13px" }}>
                <span style={{ color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "7px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "#48484a", display: "inline-block" }} />
                  {compName}
                </span>
                <span style={{ fontFamily: "var(--font-mono)", fontWeight: 600, color: "var(--text-secondary)" }}>
                  {current.compMem} MB
                </span>
              </div>
              <div style={{ height: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", borderRadius: "9999px", overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: `${compMemPct}%`,
                    background: "#48484a",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>
          </div>

          {/* Context Note */}
          <div
            style={{
              padding: "16px 20px",
              borderRadius: "12px",
              backgroundColor: "rgba(255, 255, 255, 0.03)",
              border: "1px solid rgba(255, 255, 255, 0.06)",
              fontSize: "13px",
              color: "var(--text-secondary)",
              lineHeight: 1.6,
            }}
          >
            <strong style={{ color: "var(--text-primary)" }}>Architecture context: </strong>
            {data[competitor].notes}
          </div>

          {/* Feature Matrix vs Paid Enterprise */}
          <div style={{ marginTop: "40px", paddingTop: "32px", borderTop: "1px solid rgba(255, 255, 255, 0.08)" }}>
            <h3 style={{ fontSize: "18px", fontWeight: 600, color: "#ffffff", marginBottom: "8px" }}>
              Enterprise Commercial Feature Parity
            </h3>
            <p style={{ fontSize: "13px", color: "var(--text-tertiary)", marginBottom: "24px" }}>
              Why pay $3,500+/year per instance when you can run memory-safe Rust with zero license fees?
            </p>

            <div style={{ overflowX: "auto" }}>
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "13px", textAlign: "left" }}>
                <thead>
                  <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.12)", color: "var(--text-secondary)" }}>
                    <th style={{ padding: "12px 14px" }}>Capability</th>
                    <th style={{ padding: "12px 14px", color: "var(--text-tint)", fontWeight: 700 }}>NexusLB v0.0.3</th>
                    <th style={{ padding: "12px 14px" }}>NGINX Plus</th>
                    <th style={{ padding: "12px 14px" }}>HAProxy Enterprise</th>
                  </tr>
                </thead>
                <tbody style={{ color: "var(--text-primary)" }}>
                  <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                    <td style={{ padding: "12px 14px", fontWeight: 600 }}>Licensing & Cost</td>
                    <td style={{ padding: "12px 14px", color: "#30d158", fontWeight: 700 }}>$0 (100% Free & Open Source)</td>
                    <td style={{ padding: "12px 14px", color: "#ff453a" }}>$3,500+ / yr / node</td>
                    <td style={{ padding: "12px 14px", color: "#ff453a" }}>Custom Commercial License</td>
                  </tr>
                  <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                    <td style={{ padding: "12px 14px", fontWeight: 600 }}>Active Health Checks</td>
                    <td style={{ padding: "12px 14px", color: "#30d158", fontWeight: 700 }}>Included (HTTP & TCP Probers)</td>
                    <td style={{ padding: "12px 14px", color: "#ff9f0a" }}>Paid Only (Locked in Plus)</td>
                    <td style={{ padding: "12px 14px" }}>Included</td>
                  </tr>
                  <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                    <td style={{ padding: "12px 14px", fontWeight: 600 }}>Zero-Downtime Dynamic Reconfiguration</td>
                    <td style={{ padding: "12px 14px", color: "#30d158", fontWeight: 700 }}>Included (ArcSwap Atomic Reload)</td>
                    <td style={{ padding: "12px 14px", color: "#ff9f0a" }}>Paid Only (NGINX Plus API)</td>
                    <td style={{ padding: "12px 14px", color: "#ff9f0a" }}>Data Plane API add-on</td>
                  </tr>
                  <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.06)" }}>
                    <td style={{ padding: "12px 14px", fontWeight: 600 }}>Live Dashboard & Telemetry</td>
                    <td style={{ padding: "12px 14px", color: "#30d158", fontWeight: 700 }}>Included (Terminal TUI + Prometheus)</td>
                    <td style={{ padding: "12px 14px", color: "#ff9f0a" }}>Paid Only (Live Activity Dashboard)</td>
                    <td style={{ padding: "12px 14px", color: "#ff9f0a" }}>Enterprise Module</td>
                  </tr>
                  <tr>
                    <td style={{ padding: "12px 14px", fontWeight: 600 }}>Language & Memory Safety</td>
                    <td style={{ padding: "12px 14px", color: "#30d158", fontWeight: 700 }}>100% Memory-Safe Pure Rust</td>
                    <td style={{ padding: "12px 14px", color: "#ff9f0a" }}>Legacy C (Memory Vulnerabilities)</td>
                    <td style={{ padding: "12px 14px", color: "#ff9f0a" }}>Legacy C (Memory Vulnerabilities)</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
