import { Navbar } from "@/components/Navbar";
import Footer from "@/components/Footer";
import type { Metadata } from "next";
import Link from "next/link";
import {
  ShieldAlert,
  Lock,
  Cpu,
  Terminal,
  CheckCircle,
  AlertOctagon,
  KeyRound,
  ArrowLeft,
  FileCode,
} from "lucide-react";

export const metadata: Metadata = {
  title: "Product Security — NexusLB",
  description:
    "NexusLB platform security architecture, memory safety guarantees, and coordinated vulnerability disclosure.",
};

export default function SecurityPage() {
  const securityFeatures = [
    {
      icon: <Cpu size={24} color="#2997ff" />,
      title: "100% Safe Rust Core",
      desc: "Zero manual pointer arithmetic. The Rust type system and borrow checker mathematically eliminate buffer overflow, use-after-free, and data races at compile time.",
    },
    {
      icon: <Lock size={24} color="#30d158" />,
      title: "Hardened TLS 1.3 Dataplane",
      desc: "Native rustls cryptographic backend without OpenSSL C legacy vulnerabilities. Strictly enforces TLS 1.3 and TLS 1.2 with secure modern cipher suites.",
    },
    {
      icon: <ShieldAlert size={24} color="#ff9f0a" />,
      title: "Privilege Separation & Sandbox",
      desc: "Runs with minimal Linux capabilities (CAP_NET_BIND_SERVICE). Drops root immediately after binding port 80/443 and supports strict seccomp filtering.",
    },
    {
      icon: <KeyRound size={24} color="#bf5af2" />,
      title: "Constant-Time Verifications",
      desc: "JWT HMAC-SHA256 signature verification and API token comparisons use subtle constant-time primitives to resist side-channel timing attacks.",
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
            "radial-gradient(circle, rgba(0, 113, 227, 0.15) 0%, rgba(191, 90, 242, 0.04) 50%, transparent 75%)",
          filter: "blur(90px)",
          pointerEvents: "none",
          zIndex: 0,
        }}
      />

      <Navbar />

      <main style={{ position: "relative", zIndex: 1, padding: "48px 24px 96px" }}>
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
                background: "rgba(0, 113, 227, 0.12)",
                border: "1px solid rgba(0, 113, 227, 0.25)",
                fontSize: "12px",
                fontWeight: 600,
                color: "#2997ff",
                marginBottom: "20px",
              }}
            >
              <ShieldAlert size={14} />
              Platform Security Architecture
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
              Security. Defended by mathematics.
            </h1>
            <p
              style={{
                fontSize: "clamp(1.05rem, 2vw, 1.25rem)",
                color: "var(--text-secondary)",
                lineHeight: 1.5,
                maxWidth: 720,
              }}
            >
              In production edge routing, security cannot be an afterthought. NexusLB replaces decades of legacy C memory vulnerabilities with modern Rust memory safety, rigorous fuzz testing, and minimal attack surface.
            </p>
          </div>

          {/* Core Pillars */}
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 280px), 1fr))",
              gap: "20px",
              marginBottom: "56px",
            }}
          >
            {securityFeatures.map((f, idx) => (
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
                  {f.icon}
                </div>
                <h3 style={{ fontSize: "17px", fontWeight: 600, color: "#ffffff", margin: 0 }}>
                  {f.title}
                </h3>
                <p
                  style={{
                    fontSize: "14px",
                    color: "var(--text-secondary)",
                    lineHeight: 1.6,
                    margin: 0,
                  }}
                >
                  {f.desc}
                </p>
              </div>
            ))}
          </div>

          {/* Coordinated Disclosure Box */}
          <div
            className="apple-card"
            style={{
              padding: "36px",
              marginBottom: "48px",
              border: "1px solid rgba(255, 159, 10, 0.25)",
              background:
                "linear-gradient(180deg, rgba(22, 24, 34, 0.8) 0%, rgba(15, 17, 23, 0.95) 100%)",
            }}
          >
            <h2
              style={{
                fontSize: "22px",
                fontWeight: 700,
                color: "#ffffff",
                letterSpacing: "-0.02em",
                marginBottom: "14px",
                display: "flex",
                alignItems: "center",
                gap: "10px",
              }}
            >
              <AlertOctagon size={22} color="#ff9f0a" />
              Coordinated Vulnerability Disclosure Process
            </h2>
            <p style={{ fontSize: "14px", color: "var(--text-secondary)", lineHeight: 1.6, marginBottom: "20px" }}>
              We welcome security researchers and developers to audit NexusLB. If you discover a potential security
              vulnerability or memory leakage in the NexusLB codebase, please do not file a public GitHub issue.
              Instead, disclose it confidentially to our engineering team.
            </p>

            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))",
                gap: "16px",
                marginBottom: "24px",
              }}
            >
              <div
                style={{
                  padding: "16px",
                  borderRadius: "10px",
                  background: "rgba(0, 0, 0, 0.3)",
                  border: "1px solid var(--border-subtle)",
                }}
              >
                <div style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: 4 }}>Response SLA</div>
                <div style={{ fontSize: "16px", fontWeight: 700, color: "#30d158" }}>Within 24 Hours</div>
              </div>
              <div
                style={{
                  padding: "16px",
                  borderRadius: "10px",
                  background: "rgba(0, 0, 0, 0.3)",
                  border: "1px solid var(--border-subtle)",
                }}
              >
                <div style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: 4 }}>Patch Timeline</div>
                <div style={{ fontSize: "16px", fontWeight: 700, color: "#2997ff" }}>72-Hour Expedited</div>
              </div>
              <div
                style={{
                  padding: "16px",
                  borderRadius: "10px",
                  background: "rgba(0, 0, 0, 0.3)",
                  border: "1px solid var(--border-subtle)",
                }}
              >
                <div style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: 4 }}>CVE Issuance</div>
                <div style={{ fontSize: "16px", fontWeight: 700, color: "#bf5af2" }}>Authorized CNA</div>
              </div>
            </div>

            <div style={{ fontSize: "13px", color: "var(--text-secondary)" }}>
              Contact email: <code style={{ color: "#fff", background: "rgba(255,255,255,0.08)", padding: "2px 6px", borderRadius: 4 }}>security@nexuslb.dev</code>
            </div>
          </div>

          {/* Threat Modeling & Memory Safety Specs */}
          <div
            style={{
              borderTop: "1px solid var(--border-subtle)",
              paddingTop: "40px",
              display: "flex",
              flexDirection: "column",
              gap: "32px",
              fontSize: "15px",
              lineHeight: 1.7,
              color: "var(--text-secondary)",
            }}
          >
            <section>
              <h3 style={{ fontSize: "19px", fontWeight: 600, color: "#fff", marginBottom: "12px" }}>
                Compiler-Level Mitigations
              </h3>
              <p>
                The NexusLB workspace is compiled with hardened compiler flags:
              </p>
              <ul style={{ paddingLeft: "24px", marginTop: "12px", display: "flex", flexDirection: "column", gap: "8px" }}>
                <li><strong>Control Flow Integrity & ASLR:</strong> Position Independent Executable (PIE) enabled.</li>
                <li><strong>Stack Canaries:</strong> Stack overflow detection instrumentation active.</li>
                <li><strong>Panic Abort:</strong> <code style={{ color: "#2997ff" }}>panic = "abort"</code> guarantees that in catastrophic invariants, execution halts immediately with zero stack unwinding corruption.</li>
                <li><strong>Strict Clippy & Deny:</strong> CI enforces <code style={{ color: "#2997ff" }}>-D warnings</code> and audits third-party dependency trees with <code style={{ color: "#2997ff" }}>cargo audit</code>.</li>
              </ul>
            </section>
          </div>
        </div>
      </main>

      <Footer />
    </div>
  );
}
