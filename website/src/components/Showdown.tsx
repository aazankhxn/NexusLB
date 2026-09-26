"use client";

import { useState } from "react";
import { Check, Shield, Zap, Cpu, BarChart2 } from "lucide-react";

export function Showdown() {
  const [competitor, setCompetitor] = useState<"nginx" | "ultrabalancer" | "haproxy" | "envoy">("nginx");
  const [concurrency, setConcurrency] = useState<"50" | "100" | "250">("100");

  const data = {
    nginx: {
      name: "NGINX (Production v1.31)",
      "50": { nexusRps: 112518, compRps: 109160, nexusLat: 420, compLat: 580, nexusMem: 2.6, compMem: 22.7 },
      "100": { nexusRps: 111470, compRps: 107363, nexusLat: 525, compLat: 658, nexusMem: 2.6, compMem: 22.7 },
      "250": { nexusRps: 109500, compRps: 107560, nexusLat: 890, compLat: 1120, nexusMem: 2.6, compMem: 22.7 },
      notes: "NGINX uses multi-process worker pools (C codebase) with byte-by-byte parsing."
    },
    ultrabalancer: {
      name: "UltraBalancer v3.0.0",
      "50": { nexusRps: 118872, compRps: 83152, nexusLat: 720, compLat: 990, nexusMem: 12.3, compMem: 36.4 },
      "100": { nexusRps: 101329, compRps: 78296, nexusLat: 897, compLat: 1137, nexusMem: 12.3, compMem: 36.4 },
      "250": { nexusRps: 102589, compRps: 79764, nexusLat: 1280, compLat: 1780, nexusMem: 12.3, compMem: 36.4 },
      notes: "UltraBalancer uses standard Tokio with dynamic allocations on connection dispatch."
    },
    haproxy: {
      name: "HAProxy v2.8",
      "50": { nexusRps: 112518, compRps: 104200, nexusLat: 525, compLat: 610, nexusMem: 2.6, compMem: 18.5 },
      "100": { nexusRps: 111470, compRps: 102800, nexusLat: 610, compLat: 720, nexusMem: 2.6, compMem: 18.5 },
      "250": { nexusRps: 109500, compRps: 99400, nexusLat: 940, compLat: 1250, nexusMem: 2.6, compMem: 18.5 },
      notes: "HAProxy is an established C-based proxy with complex Lua configuration requirements."
    },
    envoy: {
      name: "Envoy Proxy v1.30",
      "50": { nexusRps: 112518, compRps: 88500, nexusLat: 525, compLat: 950, nexusMem: 2.6, compMem: 68.0 },
      "100": { nexusRps: 111470, compRps: 85200, nexusLat: 610, compLat: 1180, nexusMem: 2.6, compMem: 72.0 },
      "250": { nexusRps: 109500, compRps: 82100, nexusLat: 940, compLat: 1650, nexusMem: 2.6, compMem: 85.0 },
      notes: "Envoy is heavy C++ microservice proxy with higher baseline memory requirements."
    }
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
        <h2 className="section-title">Head-to-Head Benchmark Showdown</h2>
        <p className="section-desc">
          Benchmarked on identical Apple Silicon hardware running against identical HTTP/1.1 mock services across multiple concurrency levels.
        </p>

        {/* Master Comparison Card */}
        <div className="apple-card" style={{ maxWidth: "1000px", margin: "0 auto" }}>
          {/* Controls Bar */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              flexWrap: "wrap",
              gap: "18px",
              paddingBottom: "28px",
              borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
              marginBottom: "32px",
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
                className={`apple-segment-btn ${competitor === "nginx" ? "active" : ""}`}
                onClick={() => setCompetitor("nginx")}
              >
                vs NGINX
              </button>
              <button
                className={`apple-segment-btn ${competitor === "ultrabalancer" ? "active" : ""}`}
                onClick={() => setCompetitor("ultrabalancer")}
              >
                vs UltraBalancer
              </button>
              <button
                className={`apple-segment-btn ${competitor === "haproxy" ? "active" : ""}`}
                onClick={() => setCompetitor("haproxy")}
              >
                vs HAProxy
              </button>
              <button
                className={`apple-segment-btn ${competitor === "envoy" ? "active" : ""}`}
                onClick={() => setCompetitor("envoy")}
              >
                vs Envoy
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
          <div style={{ marginBottom: "36px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: "16px", flexWrap: "wrap", gap: "8px" }}>
              <div>
                <span style={{ fontSize: "15px", fontWeight: 600, color: "#ffffff" }}>Throughput (Requests / Second)</span>
                <span style={{ fontSize: "12px", color: "var(--text-tertiary)", marginLeft: "8px" }}>Higher is better</span>
              </div>
              <span style={{ fontSize: "13px", fontWeight: 700, color: "var(--accent-emerald)", fontFamily: "var(--font-mono)" }}>
                NexusLB +{rpsDiff}% faster
              </span>
            </div>

            {/* NexusLB item */}
            <div style={{ marginBottom: "16px" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "6px", fontSize: "13px" }}>
                <span style={{ fontWeight: 600, color: "#ffffff", display: "flex", alignItems: "center", gap: "6px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--accent-cyan)", display: "inline-block" }} />
                  NexusLB v0.0.1
                </span>
                <span style={{ fontFamily: "var(--font-mono)", fontWeight: 700, color: "var(--accent-cyan)" }}>
                  {current.nexusRps.toLocaleString()} req/s
                </span>
              </div>
              <div style={{ height: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", borderRadius: "9999px", overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: `${nexusRpsPct}%`,
                    background: "linear-gradient(90deg, #00f0ff, #0071e3)",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>

            {/* Competitor item */}
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "6px", fontSize: "13px" }}>
                <span style={{ color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "6px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "#64748b", display: "inline-block" }} />
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
                    background: "linear-gradient(90deg, #475569, #64748b)",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>
          </div>

          {/* Metric 2: Median Latency */}
          <div style={{ marginBottom: "36px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: "16px", flexWrap: "wrap", gap: "8px" }}>
              <div>
                <span style={{ fontSize: "15px", fontWeight: 600, color: "#ffffff" }}>Median Latency P50 (Microseconds)</span>
                <span style={{ fontSize: "12px", color: "var(--text-tertiary)", marginLeft: "8px" }}>Lower is better</span>
              </div>
              <span style={{ fontSize: "13px", fontWeight: 700, color: "var(--accent-cyan)", fontFamily: "var(--font-mono)" }}>
                NexusLB {latDiff}% lower latency
              </span>
            </div>

            {/* NexusLB item */}
            <div style={{ marginBottom: "16px" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "6px", fontSize: "13px" }}>
                <span style={{ fontWeight: 600, color: "#ffffff", display: "flex", alignItems: "center", gap: "6px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--accent-cyan)", display: "inline-block" }} />
                  NexusLB v0.0.1
                </span>
                <span style={{ fontFamily: "var(--font-mono)", fontWeight: 700, color: "var(--accent-cyan)" }}>
                  {current.nexusLat} µs
                </span>
              </div>
              <div style={{ height: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", borderRadius: "9999px", overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: `${nexusLatPct}%`,
                    background: "linear-gradient(90deg, #00f0ff, #5e5ce6)",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>

            {/* Competitor item */}
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "6px", fontSize: "13px" }}>
                <span style={{ color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "6px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "#64748b", display: "inline-block" }} />
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
                    background: "linear-gradient(90deg, #475569, #64748b)",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>
          </div>

          {/* Metric 3: Resident Memory */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: "16px", flexWrap: "wrap", gap: "8px" }}>
              <div>
                <span style={{ fontSize: "15px", fontWeight: 600, color: "#ffffff" }}>Resident Memory Footprint RSS (MB)</span>
                <span style={{ fontSize: "12px", color: "var(--text-tertiary)", marginLeft: "8px" }}>Lower is better</span>
              </div>
              <span style={{ fontSize: "13px", fontWeight: 700, color: "var(--accent-purple)", fontFamily: "var(--font-mono)" }}>
                NexusLB {memDiff}% leaner
              </span>
            </div>

            {/* NexusLB item */}
            <div style={{ marginBottom: "16px" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "6px", fontSize: "13px" }}>
                <span style={{ fontWeight: 600, color: "#ffffff", display: "flex", alignItems: "center", gap: "6px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--accent-purple)", display: "inline-block" }} />
                  NexusLB v0.0.1
                </span>
                <span style={{ fontFamily: "var(--font-mono)", fontWeight: 700, color: "var(--accent-purple)" }}>
                  {current.nexusMem} MB
                </span>
              </div>
              <div style={{ height: "10px", backgroundColor: "rgba(255, 255, 255, 0.06)", borderRadius: "9999px", overflow: "hidden" }}>
                <div
                  style={{
                    height: "100%",
                    width: `${nexusMemPct}%`,
                    background: "linear-gradient(90deg, #bf5af2, #ff375f)",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>

            {/* Competitor item */}
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "6px", fontSize: "13px" }}>
                <span style={{ color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "6px" }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "#64748b", display: "inline-block" }} />
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
                    background: "linear-gradient(90deg, #475569, #64748b)",
                    borderRadius: "9999px",
                    transition: "width 0.5s var(--spring-snappy)",
                  }}
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
