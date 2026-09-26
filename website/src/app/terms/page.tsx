import { Navbar } from "@/components/Navbar";
import Footer from "@/components/Footer";
import type { Metadata } from "next";
import Link from "next/link";
import { Scale, ArrowLeft, FileCheck, CheckCircle2, ShieldCheck } from "lucide-react";

export const metadata: Metadata = {
  title: "Terms of Service & Licensing — NexusLB",
  description:
    "Terms of service, open-source governance, and licensing terms for the NexusLB project and website.",
};

export default function TermsPage() {
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
            "radial-gradient(circle, rgba(0, 113, 227, 0.12) 0%, rgba(191, 90, 242, 0.04) 50%, transparent 75%)",
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
              <Scale size={14} />
              Open Source Governance & Legal
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
              Terms of Use. Clear. Honest. Simple.
            </h1>
            <p
              style={{
                fontSize: "clamp(1.05rem, 2vw, 1.25rem)",
                color: "var(--text-secondary)",
                lineHeight: 1.5,
                maxWidth: 720,
              }}
            >
              NexusLB is free and open-source software built for high-performance infrastructure engineers.
              We believe in transparent, straightforward licensing without predatory commercial restrictions.
            </p>
          </div>

          {/* Licensing Card */}
          <div
            className="apple-card"
            style={{
              padding: "36px",
              marginBottom: "48px",
              border: "1px solid rgba(255, 255, 255, 0.12)",
              background:
                "linear-gradient(180deg, rgba(22, 24, 34, 0.8) 0%, rgba(15, 17, 23, 0.95) 100%)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "16px" }}>
              <div
                style={{
                  width: "36px",
                  height: "36px",
                  borderRadius: "10px",
                  background: "rgba(48, 209, 88, 0.15)",
                  color: "#30d158",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <FileCheck size={20} />
              </div>
              <div>
                <h2 style={{ fontSize: "20px", fontWeight: 700, color: "#fff", margin: 0 }}>
                  Dual License: Apache 2.0 & MIT
                </h2>
                <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                  Permissive Free Software Foundation & OSI Approved
                </div>
              </div>
            </div>

            <p style={{ fontSize: "14px", color: "var(--text-secondary)", lineHeight: 1.6, marginBottom: "20px" }}>
              NexusLB is distributed under the terms of both the <strong>Apache License (Version 2.0)</strong> and
              the <strong>MIT License</strong>. At your option, you may use, copy, modify, merge, publish, distribute,
              sublicense, and/or sell copies of the software under either license.
            </p>

            <div style={{ display: "flex", gap: "12px", flexWrap: "wrap" }}>
              <div
                style={{
                  padding: "10px 16px",
                  borderRadius: "8px",
                  background: "rgba(0, 0, 0, 0.3)",
                  border: "1px solid var(--border-subtle)",
                  fontSize: "13px",
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                }}
              >
                <CheckCircle2 size={16} color="#30d158" />
                <span>Commercial use allowed</span>
              </div>
              <div
                style={{
                  padding: "10px 16px",
                  borderRadius: "8px",
                  background: "rgba(0, 0, 0, 0.3)",
                  border: "1px solid var(--border-subtle)",
                  fontSize: "13px",
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                }}
              >
                <CheckCircle2 size={16} color="#30d158" />
                <span>Modification & redistribution allowed</span>
              </div>
              <div
                style={{
                  padding: "10px 16px",
                  borderRadius: "8px",
                  background: "rgba(0, 0, 0, 0.3)",
                  border: "1px solid var(--border-subtle)",
                  fontSize: "13px",
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                }}
              >
                <CheckCircle2 size={16} color="#30d158" />
                <span>Private & proprietary use allowed</span>
              </div>
            </div>
          </div>

          {/* Legal Sections */}
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
                1. Disclaimer of Warranties
              </h3>
              <p>
                NexusLB is provided on an &quot;AS IS&quot; BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND,
                either express or implied, including, without limitation, any warranties or conditions of TITLE,
                NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A PARTICULAR PURPOSE. You are solely responsible
                for determining the appropriateness of using or redistributing the software and assume any risks
                associated with your exercise of permissions under the License.
              </p>
            </section>

            <section>
              <h3 style={{ fontSize: "19px", fontWeight: 600, color: "#fff", marginBottom: "12px" }}>
                2. Limitation of Liability
              </h3>
              <p>
                In no event and under no legal theory, whether in tort (including negligence), contract, or otherwise,
                unless required by applicable law (such as deliberate and grossly negligent acts) or agreed to in writing,
                shall any contributor be liable to you for damages, including any direct, indirect, special, incidental,
                or consequential damages of any character arising as a result of this License or out of the use or inability
                to use the software.
              </p>
            </section>

            <section>
              <h3 style={{ fontSize: "19px", fontWeight: 600, color: "#fff", marginBottom: "12px" }}>
                3. Acceptable Use of Documentation & Website
              </h3>
              <p>
                You may access, read, and download all documentation and benchmark scripts for internal operational
                evaluation or publication. You agree not to perform denial-of-service attacks or malicious automated
                scraping against the NexusLB public web infrastructure.
              </p>
              <div style={{ marginTop: "16px", fontSize: "13px", color: "var(--text-muted)" }}>
                Last revised: September 2026. Official NexusLB Project Legal Repository.
              </div>
            </section>
          </div>
        </div>
      </main>

      <Footer />
    </div>
  );
}
