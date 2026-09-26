"use client";

import { useState } from "react";
import { Copy, Check, Sliders, Code2 } from "lucide-react";

export function ConfigBuilder() {
  const [port, setPort] = useState("8080");
  const [algo, setAlgo] = useState("adaptive");
  const [workers, setWorkers] = useState("auto");
  const [tlsEnabled, setTlsEnabled] = useState(true);
  const [jwtAuth, setJwtAuth] = useState(true);
  const [rateLimit, setRateLimit] = useState(true);
  const [accessLog, setAccessLog] = useState(true);
  const [copied, setCopied] = useState(false);

  const generateYaml = () => {
    let yaml = `server:
  listen:
    - "0.0.0.0:${port}"
  workers: "${workers}"
  engine: "tokio"        # "tokio", "io-uring", or "xdp"
  reuse_port: true
  tcp_nodelay: true

load_balancer:
  algorithm: "${algo}"
  default_pool: "api-cluster"

backends:
  - name: "srv-01"
    address: "10.0.1.10:8080"
    weight: 100
    protocol: "http1"
    pool: "api-cluster"
  - name: "srv-02"
    address: "10.0.1.11:8080"
    weight: 100
    protocol: "http1"
    pool: "api-cluster"

health_check:
  enabled: true
  interval: "5s"
  timeout: "2s"
  http_path: "/healthz"
  expected_status: 200\n`;

    if (tlsEnabled) {
      yaml += `\ntls:
  enabled: true
  cert_path: "/etc/nexuslb/certs/fullchain.pem"
  key_path: "/etc/nexuslb/certs/privkey.pem"
  redirect_http_to_https: true\n`;
    }

    if (jwtAuth) {
      yaml += `\nroutes:
  - name: "api-secure"
    path: "/api/*"
    pool: "api-cluster"
    filters:
      jwt_secret: "prod-secret-token"
      add_headers:
        X-Proxy: "NexusLB"
      remove_headers:
        - "X-Internal-Secret"\n`;
    }

    if (rateLimit) {
      yaml += `\nrate_limit:
  enabled: true
  global_rps: 120000
  client_rps: 1500\n`;
    }

    if (accessLog) {
      yaml += `\naccess_log:
  enabled: true
  format: "json"
  target: "stdout"\n`;
    }

    yaml += `\nadmin:
  enabled: true
  address: "127.0.0.1:9091"

metrics:
  enabled: true
  address: "0.0.0.0:9090"`;

    return yaml;
  };

  const copyToClipboard = () => {
    navigator.clipboard.writeText(generateYaml());
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <section id="config" className="section" style={{ position: "relative" }}>
      <div className="container">
        <div className="section-label">Interactive Generator</div>
        <h2 className="section-title">NexusLB Configuration Builder</h2>
        <p className="section-desc">
          Customize routing, protocols, algorithms, and security filters, then copy a production-ready <code>nexuslb.yaml</code>.
        </p>

        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 320px), 1fr))",
            gap: "32px",
            maxWidth: "1100px",
            margin: "0 auto",
          }}
        >
          {/* Controls Form Card */}
          <div
            className="apple-card"
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "22px",
              padding: "36px 30px",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
              <Sliders size={18} color="var(--text-tint)" />
              <span style={{ fontSize: "13px", fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "#ffffff" }}>
                Runtime Settings
              </span>
            </div>

            {/* Port */}
            <div>
              <label style={{ display: "block", fontSize: "12px", fontWeight: 600, color: "var(--text-tertiary)", marginBottom: "7px", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                Listen Port
              </label>
              <input
                type="text"
                value={port}
                onChange={(e) => setPort(e.target.value)}
                style={{
                  width: "100%",
                  padding: "11px 14px",
                  borderRadius: "var(--radius-sm)",
                  backgroundColor: "rgba(0, 0, 0, 0.45)",
                  border: "1px solid rgba(255, 255, 255, 0.12)",
                  color: "#ffffff",
                  fontSize: "14px",
                  fontFamily: "var(--font-mono)",
                  outline: "none",
                }}
              />
            </div>

            {/* Algorithm */}
            <div>
              <label style={{ display: "block", fontSize: "12px", fontWeight: 600, color: "var(--text-tertiary)", marginBottom: "7px", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                Scheduling Algorithm
              </label>
              <select
                value={algo}
                onChange={(e) => setAlgo(e.target.value)}
                style={{
                  width: "100%",
                  padding: "11px 14px",
                  borderRadius: "var(--radius-sm)",
                  backgroundColor: "rgba(0, 0, 0, 0.45)",
                  border: "1px solid rgba(255, 255, 255, 0.12)",
                  color: "#ffffff",
                  fontSize: "14px",
                  outline: "none",
                }}
              >
                <option value="adaptive">adaptive (Latency + Load penalty)</option>
                <option value="power_of_two_choices">power_of_two_choices (P2C)</option>
                <option value="least_connections">least_connections</option>
                <option value="round_robin">round_robin</option>
                <option value="ewma_latency">ewma_latency (Peak EWMA)</option>
                <option value="consistent_hash">consistent_hash (Ketama ring)</option>
                <option value="ip_hash">ip_hash (Client affinity)</option>
              </select>
            </div>

            {/* Workers */}
            <div>
              <label style={{ display: "block", fontSize: "12px", fontWeight: 600, color: "var(--text-tertiary)", marginBottom: "7px", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                Worker Threads
              </label>
              <input
                type="text"
                value={workers}
                onChange={(e) => setWorkers(e.target.value)}
                style={{
                  width: "100%",
                  padding: "11px 14px",
                  borderRadius: "var(--radius-sm)",
                  backgroundColor: "rgba(0, 0, 0, 0.45)",
                  border: "1px solid rgba(255, 255, 255, 0.12)",
                  color: "#ffffff",
                  fontSize: "14px",
                  fontFamily: "var(--font-mono)",
                  outline: "none",
                }}
              />
            </div>

            {/* Toggles */}
            <div style={{ display: "flex", flexDirection: "column", gap: "16px", paddingTop: "12px", borderTop: "1px solid rgba(255, 255, 255, 0.08)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ fontSize: "13.5px", fontWeight: 600, color: "#ffffff" }}>TLS Termination & SNI</div>
                  <div style={{ fontSize: "11.5px", color: "var(--text-tertiary)" }}>Includes HTTP-&gt;HTTPS 301 redirect</div>
                </div>
                <label className="ios-switch">
                  <input type="checkbox" checked={tlsEnabled} onChange={(e) => setTlsEnabled(e.target.checked)} />
                  <span className="ios-slider"></span>
                </label>
              </div>

              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ fontSize: "13.5px", fontWeight: 600, color: "#ffffff" }}>Route JWT Authentication</div>
                  <div style={{ fontSize: "11.5px", color: "var(--text-tertiary)" }}>Validates Bearer tokens on /api/*</div>
                </div>
                <label className="ios-switch">
                  <input type="checkbox" checked={jwtAuth} onChange={(e) => setJwtAuth(e.target.checked)} />
                  <span className="ios-slider"></span>
                </label>
              </div>

              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ fontSize: "13.5px", fontWeight: 600, color: "#ffffff" }}>Sliding-Window Rate Limiting</div>
                  <div style={{ fontSize: "11.5px", color: "var(--text-tertiary)" }}>120,000 global, 1,500/client RPS</div>
                </div>
                <label className="ios-switch">
                  <input type="checkbox" checked={rateLimit} onChange={(e) => setRateLimit(e.target.checked)} />
                  <span className="ios-slider"></span>
                </label>
              </div>

              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ fontSize: "13.5px", fontWeight: 600, color: "#ffffff" }}>Non-Blocking JSON Logging</div>
                  <div style={{ fontSize: "11.5px", color: "var(--text-tertiary)" }}>128k ring buffer background thread</div>
                </div>
                <label className="ios-switch">
                  <input type="checkbox" checked={accessLog} onChange={(e) => setAccessLog(e.target.checked)} />
                  <span className="ios-slider"></span>
                </label>
              </div>
            </div>
          </div>

          {/* Generated Preview Card */}
          <div
            style={{
              backgroundColor: "#0d0e14",
              borderRadius: "var(--radius-lg)",
              border: "1px solid rgba(255, 255, 255, 0.1)",
              boxShadow: "var(--shadow-apple)",
              display: "flex",
              flexDirection: "column",
              overflow: "hidden",
            }}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                padding: "16px 22px",
                backgroundColor: "rgba(255, 255, 255, 0.03)",
                borderBottom: "1px solid rgba(255, 255, 255, 0.07)",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <Code2 size={16} color="var(--text-tint)" />
                <span style={{ fontSize: "12px", fontFamily: "var(--font-mono)", color: "var(--text-secondary)" }}>
                  nexuslb.yaml
                </span>
              </div>
              <button
                className="apple-btn apple-btn-secondary apple-btn-sm"
                onClick={copyToClipboard}
              >
                {copied ? <Check size={14} color="#30d158" /> : <Copy size={14} />}
                {copied ? "Copied!" : "Copy YAML"}
              </button>
            </div>

            <pre
              style={{
                padding: "26px",
                fontFamily: "var(--font-mono)",
                fontSize: "12.5px",
                lineHeight: 1.65,
                color: "#e5e5ea",
                overflowY: "auto",
                overflowX: "auto",
                maxHeight: "540px",
                whiteSpace: "pre",
              }}
            >
              {generateYaml()}
            </pre>
          </div>
        </div>
      </div>
    </section>
  );
}
