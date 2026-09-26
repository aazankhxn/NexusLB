import { Navbar } from "@/components/Navbar";
import Footer from "@/components/Footer";
import type { Metadata } from "next";
import Link from "next/link";
import {
  ShieldCheck,
  Lock,
  EyeOff,
  Server,
  HardDrive,
  Cpu,
  ArrowLeft,
  CheckCircle2,
  FileText,
  Key,
} from "lucide-react";

export const metadata: Metadata = {
  title: "Privacy Policy — NexusLB",
  description:
    "Privacy is a fundamental human right. NexusLB is built on zero telemetry, zero tracking, and local-first memory-safe architecture.",
};

export default function PrivacyPage() {
  const privacyPillars = [
    {
      icon: <EyeOff size={24} color="#30d158" />,
      title: "Zero Remote Telemetry",
      desc: "NexusLB contains no analytics SDKs, no phone-home mechanisms, and no remote heartbeat pings. Your traffic metadata never leaves your infrastructure.",
    },
    {
      icon: <HardDrive size={24} color="#2997ff" />,
      title: "Ephemeral Memory Only",
      desc: "Zero-allocation byte-slice streaming handles HTTP requests entirely within pre-allocated RAM ring buffers. No request or response bodies are written to unencrypted disk.",
    },
    {
      icon: <Lock size={24} color="#bf5af2" />,
      title: "Cryptographic Isolation",
      desc: "Hardware-accelerated TLS 1.3 termination with ephemeral forward secrecy (P-256, X25519). Private keys are loaded strictly into memory and never logged.",
    },
    {
      icon: <Server size={24} color="#ff9f0a" />,
      title: "100% Air-Gapped Operation",
      desc: "NexusLB requires zero internet access to validate licenses, boot, route traffic, or export metrics. It operates flawlessly inside isolated VPCs and defense-grade enclaves.",
    },
  ];

  return (
    <div style={{ minHeight: "100vh", position: "relative", overflowX: "hidden" }}>
      {/* Dynamic Background Glows */}
      <div
        style={{
          position: "fixed",
          top: "-200px",
          left: "50%",
          transform: "translateX(-50%)",
          width: "900px",
          height: "600px",
          background:
            "radial-gradient(circle, rgba(48, 209, 88, 0.12) 0%, rgba(0, 113, 227, 0.04) 50%, transparent 75%)",
          filter: "blur(90px)",
          pointerEvents: "none",
          zIndex: 0,
        }}
      />

      <Navbar />

      <main style={{ position: "relative", zIndex: 1, padding: "64px 28px 120px" }}>
        <div style={{ maxWidth: 860, margin: "0 auto" }}>
          {/* Back Navigation */}
          <div style={{ marginBottom: "32px" }}>
            <Link
              href="/"
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: "8px",
                fontSize: "13px",
                color: "var(--text-secondary)",
                textDecoration: "none",
                transition: "color 0.2s ease",
              }}
            >
              <ArrowLeft size={14} />
              Back to NexusLB Home
            </Link>
          </div>

          {/* Hero Header */}
          <div style={{ marginBottom: "56px" }}>
            <div
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: "8px",
                padding: "4px 12px",
                borderRadius: "100px",
                background: "rgba(48, 209, 88, 0.12)",
                border: "1px solid rgba(48, 209, 88, 0.25)",
                fontSize: "12px",
                fontWeight: 600,
                color: "#30d158",
                marginBottom: "20px",
              }}
            >
              <ShieldCheck size={14} />
              Apple Privacy Standard
            </div>
            <h1
              style={{
                fontSize: "clamp(2.5rem, 5vw, 4rem)",
                fontWeight: 800,
                letterSpacing: "-0.04em",
                lineHeight: 1.05,
                color: "#ffffff",
                marginBottom: "18px",
              }}
            >
              Privacy. It’s built into every byte.
            </h1>
            <p
              style={{
                fontSize: "clamp(1.05rem, 2vw, 1.25rem)",
                color: "var(--text-secondary)",
                lineHeight: 1.5,
                maxWidth: 720,
              }}
            >
              We believe privacy is a fundamental human right. NexusLB is architected from the bare metal to ensure your data, your clients’ traffic, and your network telemetry remain exclusively yours.
            </p>
          </div>

          {/* Apple Privacy Nutrition Label Card */}
          <div
            className="apple-card"
            style={{
              padding: "36px",
              marginBottom: "48px",
              border: "1px solid rgba(48, 209, 88, 0.2)",
              background:
                "linear-gradient(180deg, rgba(22, 24, 34, 0.8) 0%, rgba(15, 17, 23, 0.95) 100%)",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                paddingBottom: "24px",
                borderBottom: "1px solid var(--border-subtle)",
                marginBottom: "24px",
                flexWrap: "wrap",
                gap: "12px",
              }}
            >
              <div>
                <div
                  style={{
                    fontSize: "11px",
                    fontWeight: 700,
                    textTransform: "uppercase",
                    letterSpacing: "0.1em",
                    color: "#30d158",
                    marginBottom: 4,
                  }}
                >
                  Privacy Details
                </div>
                <h2 style={{ fontSize: "22px", fontWeight: 700, color: "#fff", margin: 0 }}>
                  App Privacy Nutrition Summary
                </h2>
              </div>
              <div
                style={{
                  fontSize: "12px",
                  padding: "6px 12px",
                  borderRadius: "100px",
                  background: "rgba(255, 255, 255, 0.06)",
                  border: "1px solid rgba(255, 255, 255, 0.1)",
                  color: "var(--text-secondary)",
                }}
              >
                Official Verification: 0 Telemetry Traces
              </div>
            </div>

            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
                gap: "24px",
              }}
            >
              <div
                style={{
                  padding: "20px",
                  borderRadius: "14px",
                  background: "rgba(0, 0, 0, 0.3)",
                  border: "1px solid var(--border-subtle)",
                }}
              >
                <div style={{ color: "#30d158", marginBottom: "8px" }}>
                  <CheckCircle2 size={24} />
                </div>
                <div style={{ fontSize: "16px", fontWeight: 700, color: "#fff", marginBottom: 6 }}>
                  Data Not Collected
                </div>
                <p style={{ fontSize: "13px", color: "var(--text-secondary)", margin: 0, lineHeight: 1.5 }}>
                  NexusLB does not collect any data or metrics from this proxy binary or website.
                </p>
              </div>

              <div
                style={{
                  padding: "20px",
                  borderRadius: "14px",
                  background: "rgba(0, 0, 0, 0.3)",
                  border: "1px solid var(--border-subtle)",
                }}
              >
                <div style={{ color: "#30d158", marginBottom: "8px" }}>
                  <CheckCircle2 size={24} />
                </div>
                <div style={{ fontSize: "16px", fontWeight: 700, color: "#fff", marginBottom: 6 }}>
                  Data Not Linked to You
                </div>
                <p style={{ fontSize: "13px", color: "var(--text-secondary)", margin: 0, lineHeight: 1.5 }}>
                  Zero client identity, IP address, user-agent, or geolocation records are retained externally.
                </p>
              </div>

              <div
                style={{
                  padding: "20px",
                  borderRadius: "14px",
                  background: "rgba(0, 0, 0, 0.3)",
                  border: "1px solid var(--border-subtle)",
                }}
              >
                <div style={{ color: "#30d158", marginBottom: "8px" }}>
                  <CheckCircle2 size={24} />
                </div>
                <div style={{ fontSize: "16px", fontWeight: 700, color: "#fff", marginBottom: 6 }}>
                  Data Used to Track: None
                </div>
                <p style={{ fontSize: "13px", color: "var(--text-secondary)", margin: 0, lineHeight: 1.5 }}>
                  Zero tracking identifiers, cookies, or device fingerprinting routines are embedded.
                </p>
              </div>
            </div>
          </div>

          {/* Privacy Architecture Grid */}
          <div style={{ marginBottom: "56px" }}>
            <h2
              style={{
                fontSize: "24px",
                fontWeight: 700,
                color: "#ffffff",
                letterSpacing: "-0.02em",
                marginBottom: "24px",
              }}
            >
              Architectural Guarantees
            </h2>

            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 280px), 1fr))",
                gap: "20px",
              }}
            >
              {privacyPillars.map((p, idx) => (
                <div
                  key={idx}
                  className="apple-card"
                  style={{
                    padding: "28px",
                    display: "flex",
                    flexDirection: "column",
                    gap: "14px",
                  }}
                >
                  <div
                    style={{
                      width: "44px",
                      height: "44px",
                      borderRadius: "12px",
                      background: "rgba(255, 255, 255, 0.05)",
                      border: "1px solid rgba(255, 255, 255, 0.08)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                    }}
                  >
                    {p.icon}
                  </div>
                  <h3 style={{ fontSize: "17px", fontWeight: 600, color: "#ffffff", margin: 0 }}>
                    {p.title}
                  </h3>
                  <p
                    style={{
                      fontSize: "14px",
                      color: "var(--text-secondary)",
                      lineHeight: 1.6,
                      margin: 0,
                    }}
                  >
                    {p.desc}
                  </p>
                </div>
              ))}
            </div>
          </div>

          {/* Detailed Policy Text */}
          <div
            style={{
              borderTop: "1px solid var(--border-subtle)",
              paddingTop: "40px",
              display: "flex",
              flexDirection: "column",
              gap: "36px",
              fontSize: "15px",
              lineHeight: 1.7,
              color: "var(--text-secondary)",
            }}
          >
            <section>
              <h3 style={{ fontSize: "19px", fontWeight: 600, color: "#fff", marginBottom: "12px" }}>
                1. Information NexusLB Does Not Collect
              </h3>
              <p>
                Unlike many commercial reverse proxies and cloud ingress controllers, NexusLB does not include
                any proprietary telemetry probes, diagnostic crash reporters, license checking callbacks, or
                usage tracking. When you download and run the compiled NexusLB binary, it establishes network
                connections <strong>strictly and exclusively</strong> to the listener addresses and upstream
                backend targets that you define in your configuration file.
              </p>
            </section>

            <section>
              <h3 style={{ fontSize: "19px", fontWeight: 600, color: "#fff", marginBottom: "12px" }}>
                2. HTTP Traffic & Zero-Allocation Ingress
              </h3>
              <p>
                NexusLB operates as an in-memory streaming proxy using zero-allocation byte slices and pre-allocated
                circular ring buffers. When an HTTP/1.1 or HTTP/2 request is parsed via SIMD-accelerated httparse,
                header mutations and URI rewrites happen strictly in volatile RAM. No request payloads, body
                streams, cookies, or authorization tokens are persisted to local disk, caching drives, or external
                logging servers unless you explicitly configure the local access logging filter.
              </p>
            </section>

            <section>
              <h3 style={{ fontSize: "19px", fontWeight: 600, color: "#fff", marginBottom: "12px" }}>
                3. Local Access Logs & Metrics Storage
              </h3>
              <p>
                When access logging is enabled in your configuration (`nexuslb.yaml`), log records (timestamp,
                method, path, status, latency) are written directly to your local file descriptor or stdout.
                NexusLB provides a native Prometheus metrics endpoint (`/metrics`) and an operator TUI (`nexuslb top`).
                All metrics are computed via atomic hardware registers (`AtomicU64`) within memory. No external metric
                aggregators receive this data unless scraped by your own authenticated monitoring system.
              </p>
            </section>

            <section>
              <h3 style={{ fontSize: "19px", fontWeight: 600, color: "#fff", marginBottom: "12px" }}>
                4. Compliance by Architecture (GDPR, CCPA, HIPAA)
              </h3>
              <p>
                Because NexusLB does not store, process, or transmit personal data to any third parties, it
                inherently complies with the strictest global privacy regulations:
              </p>
              <ul style={{ paddingLeft: "24px", marginTop: "12px", display: "flex", flexDirection: "column", gap: "8px" }}>
                <li><strong>GDPR (EU):</strong> NexusLB acts purely as an agnostic data pipeline. It requires no Data Processing Addendum (DPA) because zero personal data is gathered by the project or developers.</li>
                <li><strong>CCPA / CPRA (California):</strong> Zero personal information is sold, shared, or collected.</li>
                <li><strong>HIPAA (Healthcare):</strong> Safe Rust memory guarantees prevent buffer overflow vulnerabilities, and end-to-end TLS 1.3 encryption prevents ePHI interception in transit.</li>
              </ul>
            </section>

            <section>
              <h3 style={{ fontSize: "19px", fontWeight: 600, color: "#fff", marginBottom: "12px" }}>
                5. Website & Documentation Usage
              </h3>
              <p>
                This official documentation website is hosted on high-performance static infrastructure. It uses no
                tracking pixels, no marketing cookies, no Google Analytics, and no advertising beacons. Any preferences
                (such as documentation search queries or dismissal of the privacy notification pill) are stored
                exclusively in your browser’s `localStorage` and never transmitted across the network.
              </p>
            </section>

            <section>
              <h3 style={{ fontSize: "19px", fontWeight: 600, color: "#fff", marginBottom: "12px" }}>
                6. Contact & Security Inquiries
              </h3>
              <p>
                If you have questions regarding this privacy policy or would like to report a security consideration,
                please review our <Link href="/security" style={{ color: "#2997ff", textDecoration: "none" }}>Product Security</Link> documentation
                or contact the maintainers at <code style={{ color: "#fff" }}>security@nexuslb.dev</code>.
              </p>
              <div style={{ marginTop: "16px", fontSize: "13px", color: "var(--text-muted)" }}>
                Last revised: September 2026. Effective with NexusLB v1.0.0.
              </div>
            </section>
          </div>
        </div>
      </main>

      <Footer />
    </div>
  );
}
