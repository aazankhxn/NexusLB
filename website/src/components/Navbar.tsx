"use client";

import Link from "next/link";
import Image from "next/image";
import { useState, useEffect } from "react";
import {
  Activity,
  Sliders,
  Layers,
  FileCode2,
  BookOpen,
  ArrowUpRight,
  Menu,
  X,
  ChevronRight,
} from "lucide-react";

function GithubIcon({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
    </svg>
  );
}

export function Navbar() {
  const [scrolled, setScrolled] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  useEffect(() => {
    const handleScroll = () => {
      setScrolled(window.scrollY > 15);
    };
    window.addEventListener("scroll", handleScroll);
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  // Prevent background scrolling when mobile menu is open
  useEffect(() => {
    if (mobileMenuOpen) {
      document.body.style.overflow = "hidden";
    } else {
      document.body.style.overflow = "";
    }
  }, [mobileMenuOpen]);

  const navItems = [
    {
      title: "Benchmarks",
      subtitle: "Empirical vs NGINX, HAProxy, Envoy",
      href: "/#showdown",
      icon: Activity,
    },
    {
      title: "Simulator",
      subtitle: "Dynamic Adaptive Traffic Steering",
      href: "/#simulator",
      icon: Sliders,
    },
    {
      title: "Architecture",
      subtitle: "Lock-Free Zero-Alloc Dataplane",
      href: "/#architecture",
      icon: Layers,
    },
    {
      title: "Config Builder",
      subtitle: "Interactive Production YAML Generator",
      href: "/#config",
      icon: FileCode2,
    },
    {
      title: "Documentation",
      subtitle: "APIs, Performance, RFC Compliance",
      href: "/docs",
      icon: BookOpen,
      accent: true,
    },
  ];

  return (
    <>
      <header
        style={{
          position: "sticky",
          top: 0,
          zIndex: 100,
          padding: "14px 20px",
          transition: "all 0.3s cubic-bezier(0.16, 1, 0.3, 1)",
        }}
      >
        <div
          style={{
            maxWidth: "1140px",
            margin: "0 auto",
            height: "56px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "0 18px",
            borderRadius: "9999px",
            backgroundColor: scrolled
              ? "rgba(14, 15, 20, 0.88)"
              : "rgba(18, 19, 26, 0.65)",
            backdropFilter: "blur(24px) saturate(180%)",
            WebkitBackdropFilter: "blur(24px) saturate(180%)",
            border: "1px solid rgba(255, 255, 255, 0.09)",
            boxShadow: scrolled
              ? "0 14px 34px rgba(0, 0, 0, 0.55), inset 0 1px 0 rgba(255, 255, 255, 0.12)"
              : "0 4px 20px rgba(0, 0, 0, 0.3)",
          }}
        >
          {/* Brand */}
          <Link
            href="/"
            style={{
              display: "flex",
              alignItems: "center",
              gap: "10px",
              textDecoration: "none",
              color: "#ffffff",
              flexShrink: 0,
            }}
            onClick={() => setMobileMenuOpen(false)}
          >
            <Image
              src="/nexuslb.png"
              alt="NexusLB"
              width={28}
              height={28}
              style={{ borderRadius: "7px", objectFit: "contain" }}
              priority
            />
            <span style={{ fontWeight: 700, fontSize: "16px", letterSpacing: "-0.025em" }}>
              NexusLB
            </span>
            <span
              style={{
                fontSize: "11px",
                fontFamily: "var(--font-mono)",
                padding: "2px 7px",
                borderRadius: "9999px",
                backgroundColor: "rgba(255, 255, 255, 0.07)",
                color: "var(--text-secondary)",
                border: "1px solid rgba(255, 255, 255, 0.1)",
                fontWeight: 500,
              }}
            >
              v0.0.1
            </span>
          </Link>

          {/* Desktop Navigation: Apple SF-Style Floating Icon Toolbar */}
          <nav
            style={{
              display: "flex",
              alignItems: "center",
              gap: "4px",
              background: "rgba(255, 255, 255, 0.04)",
              padding: "4px 8px",
              borderRadius: "9999px",
              border: "1px solid rgba(255, 255, 255, 0.06)",
            }}
            className="desktop-nav"
            aria-label="Primary Navigation"
          >
            {navItems.map((item) => {
              const IconComp = item.icon;
              return (
                <Link
                  key={item.title}
                  href={item.href}
                  className="apple-icon-nav-btn"
                  aria-label={item.title}
                  style={item.accent ? { color: "var(--text-tint)" } : undefined}
                >
                  <IconComp size={18} strokeWidth={1.9} />
                  <span className="apple-tooltip">{item.title}</span>
                </Link>
              );
            })}
          </nav>

          {/* Desktop Actions */}
          <div className="desktop-only" style={{ alignItems: "center", gap: "10px", flexShrink: 0 }}>
            <a
              href="https://github.com/aazankhxn/NexusLB"
              target="_blank"
              rel="noreferrer"
              className="apple-icon-nav-btn"
              aria-label="GitHub Repository"
              style={{ width: "36px", height: "36px" }}
            >
              <GithubIcon size={16} />
              <span className="apple-tooltip">GitHub</span>
            </a>
            <Link
              href="/docs"
              className="apple-btn apple-btn-primary apple-btn-sm"
              style={{
                height: "36px",
                padding: "0 16px",
                fontSize: "13px",
                fontWeight: 600,
              }}
            >
              Get Started
              <ArrowUpRight size={14} />
            </Link>
          </div>

          {/* Mobile Menu Hamburger Button */}
          <button
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="mobile-only"
            aria-label="Toggle Navigation Menu"
            style={{
              background: "rgba(255, 255, 255, 0.06)",
              border: "1px solid rgba(255, 255, 255, 0.12)",
              borderRadius: "50%",
              width: "38px",
              height: "38px",
              alignItems: "center",
              justifyContent: "center",
              color: "#ffffff",
              cursor: "pointer",
              transition: "transform 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
            }}
          >
            {mobileMenuOpen ? <X size={18} /> : <Menu size={18} />}
          </button>
        </div>
      </header>

      {/* Luxury Mobile Navigation Sheet / Drawer */}
      {mobileMenuOpen && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            zIndex: 99,
            backgroundColor: "rgba(0, 0, 0, 0.85)",
            backdropFilter: "blur(28px) saturate(190%)",
            WebkitBackdropFilter: "blur(28px) saturate(190%)",
            display: "flex",
            flexDirection: "column",
            padding: "84px 24px 32px",
            animation: "fadeIn 0.25s cubic-bezier(0.16, 1, 0.3, 1)",
            overflowY: "auto",
          }}
        >
          <div style={{ marginBottom: "16px" }}>
            <span
              style={{
                fontSize: "11px",
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.1em",
                color: "var(--text-tertiary)",
              }}
            >
              Navigation
            </span>
          </div>

          <nav
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "8px",
              marginBottom: "32px",
            }}
          >
            {navItems.map((item) => {
              const IconComp = item.icon;
              return (
                <Link
                  key={item.title}
                  href={item.href}
                  onClick={() => setMobileMenuOpen(false)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "12px 14px",
                    borderRadius: "14px",
                    backgroundColor: "rgba(255, 255, 255, 0.04)",
                    border: "1px solid rgba(255, 255, 255, 0.06)",
                    textDecoration: "none",
                    transition: "all 0.2s ease",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: "14px" }}>
                    <div
                      style={{
                        width: "36px",
                        height: "36px",
                        borderRadius: "10px",
                        backgroundColor: item.accent
                          ? "rgba(41, 151, 255, 0.15)"
                          : "rgba(255, 255, 255, 0.06)",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        color: item.accent ? "#2997ff" : "#ffffff",
                        flexShrink: 0,
                      }}
                    >
                      <IconComp size={18} strokeWidth={2} />
                    </div>
                    <div>
                      <div
                        style={{
                          fontSize: "15px",
                          fontWeight: 600,
                          color: "#ffffff",
                          letterSpacing: "-0.01em",
                        }}
                      >
                        {item.title}
                      </div>
                      <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                        {item.subtitle}
                      </div>
                    </div>
                  </div>
                  <ChevronRight size={16} color="var(--text-tertiary)" />
                </Link>
              );
            })}
          </nav>

          <div style={{ marginTop: "auto", display: "flex", flexDirection: "column", gap: "10px" }}>
            <Link
              href="/docs"
              onClick={() => setMobileMenuOpen(false)}
              className="apple-btn apple-btn-primary"
              style={{ height: "46px", fontSize: "15px", width: "100%" }}
            >
              Get Started with NexusLB
              <ArrowUpRight size={16} />
            </Link>
            <a
              href="https://github.com/aazankhxn/NexusLB"
              target="_blank"
              rel="noreferrer"
              className="apple-btn apple-btn-secondary"
              style={{ height: "46px", fontSize: "15px", width: "100%" }}
            >
              <GithubIcon size={16} />
              View Source on GitHub
            </a>
          </div>
        </div>
      )}
    </>
  );
}
