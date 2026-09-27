"use client";

import { useState } from "react";
import Link from "next/link";
import Image from "next/image";
import { ArrowRight, Terminal, Zap, Shield, Layers, Flame, Copy, Check } from "lucide-react";

interface SnippetLine {
  prompt?: boolean;
  text: string;
  color?: string;
  bold?: boolean;
}

export function Hero() {
  const [terminalTab, setTerminalTab] = useState<"start" | "top" | "build">("start");
  const [copied, setCopied] = useState(false);

  const snippets: Record<"start" | "top" | "build", SnippetLine[]> = {
    start: [
      { prompt: true, text: "./target/release/nexuslb start --config nexuslb.yaml" },
      { text: "2026-09-27T12:00:00Z  INFO nexuslb: Starting NexusLB v0.0.5", color: "#86868b" },
      { text: "2026-09-27T12:00:00Z  INFO nexuslb: I/O Engine: tokio [4 workers assigned]", color: "#86868b" },
      { text: "2026-09-27T12:00:00Z  INFO nexuslb_network: Listening on 0.0.0.0:8080 (SO_REUSEPORT, TCP_NODELAY)", color: "#30d158" },
      { text: "2026-09-27T12:00:00Z  INFO nexuslb_api: Admin REST API listening on 127.0.0.1:9091", color: "#30d158" },
      { text: "2026-09-27T12:00:00Z  INFO nexuslb: Ready for production traffic. Press Ctrl+C to terminate.", color: "#2997ff" },
    ],
    top: [
      { prompt: true, text: "./target/release/nexuslb top" },
      { text: "[NexusLB Operator Dashboard v0.0.5] ──────────────── Up: 21d 08h 44m", color: "#2997ff", bold: true },
      { text: "Throughput: 114,373.7 req/s   Conns: 100 active   Drop Rate: 0.00%", color: "#30d158" },
      { text: "Latency:    P50: 500 µs   P90: 850 µs   P99: 1.00 ms", color: "#f5f5f7" },
      { text: "\nActive Pool: [api-cluster] (Algorithm: Adaptive)", color: "#86868b" },
      { text: "  ● srv-1 [10.0.1.10:8080]  UP  Load: 24%  Conns: 58   Lat: 480µs  Score: 1.12", color: "#30d158" },
      { text: "  ● srv-2 [10.0.1.11:8080]  UP  Load: 26%  Conns: 64   Lat: 510µs  Score: 1.18", color: "#30d158" },
      { text: "  ● srv-3 [10.0.1.12:8080]  UP  Load: 25%  Conns: 62   Lat: 495µs  Score: 1.15", color: "#30d158" },
      { text: "  ● srv-4 [10.0.1.13:8080]  UP  Load: 25%  Conns: 66   Lat: 515µs  Score: 1.20", color: "#30d158" },
    ],
    build: [
      { prompt: true, text: "cargo build --release" },
      { text: "   Compiling nexuslb v0.0.5 (/Users/aazankhan/Personal/NexusLB)", color: "#86868b" },
      { text: "   LTO optimization: fat, codegen-units: 1, panic: abort", color: "#86868b" },
      { text: "    Finished release [optimized] target(s) in 32.77s", color: "#30d158", bold: true },
      { prompt: true, text: "./target/release/nexuslb version" },
      { text: "NexusLB v0.0.5 [Target: macos aarch64 | Engines: tokio, io-uring, xdp]", color: "#2997ff" },
    ],
  };

  const copyCode = () => {
    const raw = snippets[terminalTab].map((s) => (s.prompt ? `$ ${s.text}` : s.text)).join("\n");
    navigator.clipboard.writeText(raw);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <section
      style={{
        paddingTop: "76px",
        paddingBottom: "110px",
        textAlign: "center",
        position: "relative",
      }}
    >
      <div className="container">
        {/* Official 3D Ribbon Logo Emblem with subtle Apple halo */}
        <div style={{ display: "flex", justifyContent: "center", marginBottom: "28px" }}>
          <div
            style={{
              position: "relative",
              width: "88px",
              height: "88px",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <div
              style={{
                position: "absolute",
                inset: -8,
                borderRadius: "32px",
                background: "radial-gradient(circle, rgba(41, 151, 255, 0.2) 0%, rgba(255, 255, 255, 0.05) 50%, transparent 75%)",
                filter: "blur(18px)",
                zIndex: 0,
              }}
            />
            <Image
              src="/nexuslb.png"
              alt="NexusLB Official Logo"
              width={88}
              height={88}
              priority
              style={{
                position: "relative",
                zIndex: 1,
                objectFit: "contain",
                filter: "drop-shadow(0 16px 32px rgba(0, 0, 0, 0.6))",
              }}
            />
          </div>
        </div>

        {/* Apple Status Capsule Badge */}
        <div
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: "9px",
            padding: "6px 16px",
            borderRadius: "9999px",
            background: "rgba(255, 255, 255, 0.05)",
            border: "1px solid rgba(255, 255, 255, 0.1)",
            boxShadow: "0 2px 12px rgba(0, 0, 0, 0.25)",
            marginBottom: "32px",
          }}
        >
          <div
            style={{
              width: "7px",
              height: "7px",
              borderRadius: "50%",
              backgroundColor: "#30d158",
              boxShadow: "0 0 6px rgba(48, 209, 88, 0.6)",
            }}
          />
          <span style={{ fontSize: "13px", fontWeight: 500, color: "var(--text-secondary)" }}>
            Allocation-minimized Hot Path &bull; Pure Safe Rust &bull; Zero Buffer Overflows
          </span>
        </div>

        {/* Master Apple Display Typography */}
        <h1
          style={{
            fontSize: "clamp(42px, 6.5vw, 76px)",
            fontWeight: 800,
            letterSpacing: "-0.035em",
            lineHeight: 1.08,
            maxWidth: "960px",
            margin: "0 auto 28px",
            color: "#ffffff",
          }}
        >
          Next-Generation <br />
          <span
            style={{
              background: "linear-gradient(180deg, #ffffff 40%, #86868b 100%)",
              WebkitBackgroundClip: "text",
              WebkitTextFillColor: "transparent",
            }}
          >
            Adaptive Load Balancing.
          </span>
        </h1>

        <p
          style={{
            fontSize: "clamp(17px, 2.2vw, 21px)",
            color: "var(--text-secondary)",
            maxWidth: "760px",
            margin: "0 auto 48px",
            lineHeight: 1.55,
            fontWeight: 400,
          }}
        >
          Out-delivers NGINX Plus with <strong style={{ color: "#ffffff" }}>118,872 req/s</strong> throughput, sub-millisecond median latencies (525 µs P50), and consumes <strong style={{ color: "#ffffff" }}>88.6% less memory</strong> with atomic zero-downtime hot reloads.
        </p>

        {/* Primary CTA Buttons */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            gap: "16px",
            flexWrap: "wrap",
            marginBottom: "72px",
          }}
        >
          <a
            href="#showdown"
            className="apple-btn apple-btn-primary"
            style={{ padding: "13px 28px", fontSize: "15px" }}
          >
            Explore Benchmark Showdown
            <ArrowRight size={16} />
          </a>
          <Link
            href="/docs"
            className="apple-btn apple-btn-secondary"
            style={{ padding: "13px 28px", fontSize: "15px" }}
          >
            Read Documentation
          </Link>
        </div>

        {/* 4 Apple-style Hardware Specs Stat Cards */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 220px), 1fr))",
            gap: "20px",
            marginBottom: "72px",
          }}
        >
          <div className="apple-card" style={{ padding: "28px 24px", textAlign: "left" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "16px" }}>
              <span style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-tint)", textTransform: "uppercase", letterSpacing: "0.08em" }}>
                Peak Throughput
              </span>
              <Zap size={18} color="var(--text-tint)" />
            </div>
            <div style={{ fontSize: "clamp(32px, 4vw, 40px)", fontWeight: 800, color: "#ffffff", letterSpacing: "-0.03em", lineHeight: 1.1 }}>
              114,373
            </div>
            <div style={{ fontSize: "13px", color: "var(--text-secondary)", marginTop: "8px" }}>
              Requests/sec sustained (+6.5% vs NGINX Plus at C=100)
            </div>
          </div>

          <div className="apple-card" style={{ padding: "28px 24px", textAlign: "left" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "16px" }}>
              <span style={{ fontSize: "12px", fontWeight: 600, color: "#30d158", textTransform: "uppercase", letterSpacing: "0.08em" }}>
                Median Latency
              </span>
              <Flame size={18} color="#30d158" />
            </div>
            <div style={{ fontSize: "clamp(32px, 4vw, 40px)", fontWeight: 800, color: "#ffffff", letterSpacing: "-0.03em", lineHeight: 1.1 }}>
              500 µs
            </div>
            <div style={{ fontSize: "13px", color: "var(--text-secondary)", marginTop: "8px" }}>
              24.0% lower latency than NGINX Plus
            </div>
          </div>

          <div className="apple-card" style={{ padding: "28px 24px", textAlign: "left" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "16px" }}>
              <span style={{ fontSize: "12px", fontWeight: 600, color: "#af52de", textTransform: "uppercase", letterSpacing: "0.08em" }}>
                Resident RAM
              </span>
              <Layers size={18} color="#af52de" />
            </div>
            <div style={{ fontSize: "clamp(32px, 4vw, 40px)", fontWeight: 800, color: "#ffffff", letterSpacing: "-0.03em", lineHeight: 1.1 }}>
              11.0 MB
            </div>
            <div style={{ fontSize: "13px", color: "var(--text-secondary)", marginTop: "8px" }}>
              Flatline RSS — 51.5% leaner than NGINX Plus
            </div>
          </div>

          <div className="apple-card" style={{ padding: "28px 24px", textAlign: "left" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "16px" }}>
              <span style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-tint)", textTransform: "uppercase", letterSpacing: "0.08em" }}>
                Hot Reload Time
              </span>
              <Shield size={18} color="var(--text-tint)" />
            </div>
            <div style={{ fontSize: "clamp(32px, 4vw, 40px)", fontWeight: 800, color: "#ffffff", letterSpacing: "-0.03em", lineHeight: 1.1 }}>
              &lt; 1 µs
            </div>
            <div style={{ fontSize: "13px", color: "var(--text-secondary)", marginTop: "8px" }}>
              Lock-free ArcSwap zero dropped requests
            </div>
          </div>
        </div>

        {/* Apple macOS Terminal Preview */}
        <div
          style={{
            maxWidth: "940px",
            margin: "0 auto",
            backgroundColor: "#0d0e14",
            borderRadius: "var(--radius-md)",
            border: "1px solid rgba(255, 255, 255, 0.1)",
            boxShadow: "0 24px 60px rgba(0, 0, 0, 0.7), inset 0 1px 0 rgba(255, 255, 255, 0.1)",
            overflow: "hidden",
            textAlign: "left",
          }}
        >
          {/* Window Chrome */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "12px 18px",
              backgroundColor: "rgba(255, 255, 255, 0.03)",
              borderBottom: "1px solid rgba(255, 255, 255, 0.07)",
              flexWrap: "wrap",
              gap: "12px",
            }}
          >
            <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
              <span style={{ width: "11px", height: "11px", borderRadius: "50%", backgroundColor: "#ff5f56" }} />
              <span style={{ width: "11px", height: "11px", borderRadius: "50%", backgroundColor: "#ffbd2e" }} />
              <span style={{ width: "11px", height: "11px", borderRadius: "50%", backgroundColor: "#27c93f" }} />
            </div>

            <div
              className="apple-segmented"
              style={{
                maxWidth: "100%",
                overflowX: "auto",
                scrollbarWidth: "none",
              }}
            >
              <button
                className={`apple-segment-btn ${terminalTab === "start" ? "active" : ""}`}
                onClick={() => setTerminalTab("start")}
              >
                nexuslb start
              </button>
              <button
                className={`apple-segment-btn ${terminalTab === "top" ? "active" : ""}`}
                onClick={() => setTerminalTab("top")}
              >
                nexuslb top
              </button>
              <button
                className={`apple-segment-btn ${terminalTab === "build" ? "active" : ""}`}
                onClick={() => setTerminalTab("build")}
              >
                cargo build
              </button>
            </div>

            <button
              onClick={copyCode}
              style={{
                background: "none",
                border: "none",
                color: copied ? "#30d158" : "var(--text-tertiary)",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                gap: "5px",
                fontSize: "12px",
                fontFamily: "var(--font-mono)",
                transition: "color 0.2s",
              }}
            >
              {copied ? <Check size={14} /> : <Copy size={14} />}
              {copied ? "Copied" : "Copy"}
            </button>
          </div>

          {/* Terminal Console */}
          <div
            style={{
              padding: "26px",
              fontFamily: "var(--font-mono)",
              fontSize: "13px",
              lineHeight: 1.75,
              minHeight: "220px",
              overflowX: "auto",
              color: "#c9d1d9",
            }}
          >
            {snippets[terminalTab].map((line, idx) => (
              <div
                key={idx}
                style={{
                  color: line.color || "#f5f5f7",
                  fontWeight: line.bold ? 700 : 400,
                  whiteSpace: "pre",
                }}
              >
                {line.prompt && <span style={{ color: "#30d158", marginRight: "10px" }}>$</span>}
                {line.text}
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
