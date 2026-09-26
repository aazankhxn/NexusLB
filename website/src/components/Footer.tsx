"use client";

import React from "react";
import Image from "next/image";
import { Terminal, Shield, Zap, Cpu, BookOpen, Activity, ArrowUpRight } from "lucide-react";

function GithubIcon({ size = 14 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/>
    </svg>
  );
}

export default function Footer() {
  return (
    <footer style={{
      borderTop: "1px solid var(--border-subtle)",
      background: "rgba(10, 11, 16, 0.75)",
      backdropFilter: "blur(20px)",
      WebkitBackdropFilter: "blur(20px)",
      padding: "64px 24px 48px",
      marginTop: "96px",
      position: "relative",
      zIndex: 10,
    }}>
      <div style={{
        maxWidth: 1200,
        margin: "0 auto",
      }}>
        {/* Top Highlights Banner */}
        <div style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
          gap: "24px",
          paddingBottom: "48px",
          borderBottom: "1px solid var(--border-subtle)",
          marginBottom: "48px",
        }}>
          <div style={{ display: "flex", alignItems: "flex-start", gap: "12px" }}>
            <div style={{
              width: 36,
              height: 36,
              borderRadius: "10px",
              background: "rgba(0, 113, 227, 0.12)",
              border: "1px solid rgba(0, 113, 227, 0.25)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "#2997ff",
              flexShrink: 0
            }}>
              <Zap size={18} />
            </div>
            <div>
              <div style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)", marginBottom: 4 }}>
                Sub-Millisecond P99
              </div>
              <div style={{ fontSize: "12px", color: "var(--text-secondary)", lineHeight: 1.5 }}>
                0.41ms at 50,000 req/sec benchmarked against production hardware.
              </div>
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "flex-start", gap: "12px" }}>
            <div style={{
              width: 36,
              height: 36,
              borderRadius: "10px",
              background: "rgba(50, 215, 75, 0.12)",
              border: "1px solid rgba(50, 215, 75, 0.25)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "#30d158",
              flexShrink: 0
            }}>
              <Cpu size={18} />
            </div>
            <div>
              <div style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)", marginBottom: 4 }}>
                Zero Allocation
              </div>
              <div style={{ fontSize: "12px", color: "var(--text-secondary)", lineHeight: 1.5 }}>
                Streaming httparse buffer with byte-slice header manipulation.
              </div>
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "flex-start", gap: "12px" }}>
            <div style={{
              width: 36,
              height: 36,
              borderRadius: "10px",
              background: "rgba(191, 90, 242, 0.12)",
              border: "1px solid rgba(191, 90, 242, 0.25)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "#bf5af2",
              flexShrink: 0
            }}>
              <Shield size={18} />
            </div>
            <div>
              <div style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)", marginBottom: 4 }}>
                Memory Safe
              </div>
              <div style={{ fontSize: "12px", color: "var(--text-secondary)", lineHeight: 1.5 }}>
                100% safe Rust core. Zero GC pauses, zero buffer overflow exploits.
              </div>
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "flex-start", gap: "12px" }}>
            <div style={{
              width: 36,
              height: 36,
              borderRadius: "10px",
              background: "rgba(255, 159, 10, 0.12)",
              border: "1px solid rgba(255, 159, 10, 0.25)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "#ff9f0a",
              flexShrink: 0
            }}>
              <Activity size={18} />
            </div>
            <div>
              <div style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)", marginBottom: 4 }}>
                Active Probing
              </div>
              <div style={{ fontSize: "12px", color: "var(--text-secondary)", lineHeight: 1.5 }}>
                EWMA latency health checks with automatic sub-millisecond failover.
              </div>
            </div>
          </div>
        </div>

        {/* Footer Navigation Columns */}
        <div style={{
          display: "grid",
          gridTemplateColumns: "2fr 1fr 1fr 1fr",
          gap: "48px",
          marginBottom: "64px",
        }}>
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "14px" }}>
              <Image
                src="/nexuslb.png"
                alt="NexusLB Logo"
                width={28}
                height={28}
                style={{ borderRadius: "6px", objectFit: "contain" }}
              />
              <span style={{ fontSize: "18px", fontWeight: 700, letterSpacing: "-0.02em", color: "#fff" }}>
                NexusLB
              </span>
              <span style={{
                fontSize: "11px",
                fontWeight: 600,
                padding: "2px 7px",
                borderRadius: "100px",
                background: "rgba(0, 113, 227, 0.15)",
                color: "#2997ff",
                border: "1px solid rgba(0, 113, 227, 0.3)",
              }}>
                v0.0.1
              </span>
            </div>
            <p style={{
              fontSize: "13px",
              color: "var(--text-secondary)",
              lineHeight: 1.6,
              maxWidth: "340px",
              marginBottom: "20px",
            }}>
              Ultra-low latency Layer 7 reverse proxy and intelligent load balancer engineered in safe Rust with Tokio and zero-allocation HTTP streaming.
            </p>
            <div style={{ display: "flex", gap: "12px" }}>
              <a
                href="https://github.com"
                target="_blank"
                rel="noreferrer"
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "6px",
                  fontSize: "12px",
                  padding: "6px 12px",
                  borderRadius: "8px",
                  background: "var(--bg-card)",
                  border: "1px solid var(--border-subtle)",
                  color: "var(--text-secondary)",
                  textDecoration: "none",
                  transition: "all 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
                }}
              >
                <GithubIcon size={14} />
                GitHub Repository
                <ArrowUpRight size={12} />
              </a>
              <a
                href="#docs"
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "6px",
                  fontSize: "12px",
                  padding: "6px 12px",
                  borderRadius: "8px",
                  background: "var(--bg-card)",
                  border: "1px solid var(--border-subtle)",
                  color: "var(--text-secondary)",
                  textDecoration: "none",
                }}
              >
                <BookOpen size={14} />
                Documentation
              </a>
            </div>
          </div>

          <div>
            <div style={{ fontSize: "12px", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--text-muted)", marginBottom: "16px" }}>
              Architecture
            </div>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: "10px", fontSize: "13px" }}>
              <li><a href="#showdown" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>NGINX Showdown</a></li>
              <li><a href="#simulator" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Interactive Simulator</a></li>
              <li><a href="#features" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Zero-Alloc Dataplane</a></li>
              <li><a href="#config-builder" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Config Generator</a></li>
            </ul>
          </div>

          <div>
            <div style={{ fontSize: "12px", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--text-muted)", marginBottom: "16px" }}>
              Algorithms
            </div>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: "10px", fontSize: "13px" }}>
              <li><a href="#docs" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Peak EWMA</a></li>
              <li><a href="#docs" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Consistent Hash (Ketama)</a></li>
              <li><a href="#docs" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Power of 2 Choices (P2C)</a></li>
              <li><a href="#docs" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Adaptive Least Conn</a></li>
            </ul>
          </div>

          <div>
            <div style={{ fontSize: "12px", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--text-muted)", marginBottom: "16px" }}>
              Privacy & Legal
            </div>
            <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: "10px", fontSize: "13px" }}>
              <li><a href="/privacy" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Privacy Policy</a></li>
              <li><a href="/security" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Product Security</a></li>
              <li><a href="/terms" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Terms of Service</a></li>
              <li><a href="/terms" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Apache 2.0 / MIT</a></li>
            </ul>
          </div>
        </div>

        {/* Bottom Line */}
        <div style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          flexWrap: "wrap",
          gap: "16px",
          paddingTop: "24px",
          borderTop: "1px solid var(--border-subtle)",
          fontSize: "12px",
          color: "var(--text-muted)",
        }}>
          <div style={{ display: "flex", alignItems: "center", gap: "16px", flexWrap: "wrap" }}>
            <span>© {new Date().getFullYear()} Aazan Khan (@aazankhxn). Released under Apache 2.0 / MIT with Attribution.</span>
            <span style={{ color: "rgba(255,255,255,0.2)" }}>|</span>
            <a href="/privacy" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Privacy</a>
            <a href="/security" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Security</a>
            <a href="/terms" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Legal Terms</a>
            <a href="/docs" style={{ color: "var(--text-secondary)", textDecoration: "none" }}>Documentation</a>
          </div>
          <div>
            Designed with Apple Human Interface standards for high-performance computing.
          </div>
        </div>
      </div>
    </footer>
  );
}
