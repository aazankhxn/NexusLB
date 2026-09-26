"use client";

import Link from "next/link";
import Image from "next/image";
import { useState, useEffect } from "react";
import { ArrowUpRight, BookOpen, Menu, X } from "lucide-react";

function GithubIcon({ size = 15 }: { size?: number }) {
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

  return (
    <>
      <header
        style={{
          position: "sticky",
          top: 0,
          zIndex: 100,
          padding: "12px 16px",
          transition: "all 0.3s cubic-bezier(0.16, 1, 0.3, 1)",
        }}
      >
        <div
          style={{
            maxWidth: "1140px",
            margin: "0 auto",
            height: "54px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "0 18px",
            borderRadius: "9999px",
            backgroundColor: scrolled
              ? "rgba(18, 20, 28, 0.88)"
              : "rgba(22, 24, 34, 0.65)",
            backdropFilter: "blur(24px) saturate(180%)",
            WebkitBackdropFilter: "blur(24px) saturate(180%)",
            border: "1px solid rgba(255, 255, 255, 0.1)",
            boxShadow: scrolled
              ? "0 12px 32px rgba(0, 0, 0, 0.55), inset 0 1px 0 rgba(255, 255, 255, 0.15)"
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
            }}
            onClick={() => setMobileMenuOpen(false)}
          >
            <Image
              src="/nexuslb.png"
              alt="NexusLB"
              width={30}
              height={30}
              style={{ borderRadius: "8px", objectFit: "contain" }}
              priority
            />
            <span style={{ fontWeight: 700, fontSize: "17px", letterSpacing: "-0.02em" }}>
              NexusLB
            </span>
            <span
              style={{
                fontSize: "11px",
                fontFamily: "var(--font-mono)",
                padding: "2px 8px",
                borderRadius: "9999px",
                backgroundColor: "rgba(0, 240, 255, 0.12)",
                color: "var(--accent-cyan)",
                border: "1px solid rgba(0, 240, 255, 0.3)",
                fontWeight: 600,
              }}
            >
              v0.0.1
            </span>
          </Link>

          {/* Desktop Navigation Links */}
          <nav
            style={{
              display: "flex",
              alignItems: "center",
              gap: "28px",
            }}
            className="desktop-nav"
          >
            <Link
              href="/#showdown"
              style={{
                color: "var(--text-secondary)",
                textDecoration: "none",
                fontSize: "13.5px",
                fontWeight: 500,
                transition: "color 0.2s",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.color = "#ffffff")}
              onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-secondary)")}
            >
              Benchmarks
            </Link>
            <Link
              href="/#simulator"
              style={{
                color: "var(--text-secondary)",
                textDecoration: "none",
                fontSize: "13.5px",
                fontWeight: 500,
                transition: "color 0.2s",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.color = "#ffffff")}
              onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-secondary)")}
            >
              Simulator
            </Link>
            <Link
              href="/#architecture"
              style={{
                color: "var(--text-secondary)",
                textDecoration: "none",
                fontSize: "13.5px",
                fontWeight: 500,
                transition: "color 0.2s",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.color = "#ffffff")}
              onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-secondary)")}
            >
              Architecture
            </Link>
            <Link
              href="/#config"
              style={{
                color: "var(--text-secondary)",
                textDecoration: "none",
                fontSize: "13.5px",
                fontWeight: 500,
                transition: "color 0.2s",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.color = "#ffffff")}
              onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-secondary)")}
            >
              Config Builder
            </Link>
            <Link
              href="/docs"
              style={{
                color: "#2997ff",
                textDecoration: "none",
                fontSize: "13.5px",
                fontWeight: 600,
                display: "flex",
                alignItems: "center",
                gap: "5px",
              }}
            >
              <BookOpen size={14} />
              Documentation
            </Link>
          </nav>

          {/* Desktop Actions */}
          <div className="desktop-only" style={{ alignItems: "center", gap: "10px" }}>
            <a
              href="https://github.com/aazankhxn/NexusLB"
              target="_blank"
              rel="noreferrer"
              className="apple-btn apple-btn-secondary apple-btn-sm"
              style={{ display: "inline-flex" }}
            >
              <GithubIcon size={14} />
              GitHub
            </a>
            <Link
              href="/docs"
              className="apple-btn apple-btn-primary apple-btn-sm"
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
              background: "rgba(255, 255, 255, 0.08)",
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
            {mobileMenuOpen ? <X size={19} /> : <Menu size={19} />}
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
            backgroundColor: "rgba(0, 0, 0, 0.8)",
            backdropFilter: "blur(28px) saturate(190%)",
            WebkitBackdropFilter: "blur(28px) saturate(190%)",
            display: "flex",
            flexDirection: "column",
            padding: "88px 24px 32px",
            animation: "fadeIn 0.25s cubic-bezier(0.16, 1, 0.3, 1)",
          }}
        >
          <nav
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "20px",
              marginBottom: "36px",
            }}
          >
            <Link
              href="/#showdown"
              onClick={() => setMobileMenuOpen(false)}
              style={{
                fontSize: "20px",
                fontWeight: 600,
                color: "#ffffff",
                textDecoration: "none",
                padding: "8px 0",
                borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
              }}
            >
              Benchmarks Showdown
            </Link>
            <Link
              href="/#simulator"
              onClick={() => setMobileMenuOpen(false)}
              style={{
                fontSize: "20px",
                fontWeight: 600,
                color: "#ffffff",
                textDecoration: "none",
                padding: "8px 0",
                borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
              }}
            >
              Live Traffic Simulator
            </Link>
            <Link
              href="/#architecture"
              onClick={() => setMobileMenuOpen(false)}
              style={{
                fontSize: "20px",
                fontWeight: 600,
                color: "#ffffff",
                textDecoration: "none",
                padding: "8px 0",
                borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
              }}
            >
              Engineering Architecture
            </Link>
            <Link
              href="/#config"
              onClick={() => setMobileMenuOpen(false)}
              style={{
                fontSize: "20px",
                fontWeight: 600,
                color: "#ffffff",
                textDecoration: "none",
                padding: "8px 0",
                borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
              }}
            >
              Config Builder
            </Link>
            <Link
              href="/docs"
              onClick={() => setMobileMenuOpen(false)}
              style={{
                fontSize: "20px",
                fontWeight: 600,
                color: "#2997ff",
                textDecoration: "none",
                padding: "8px 0",
                display: "flex",
                alignItems: "center",
                gap: "8px",
                borderBottom: "1px solid rgba(0, 113, 227, 0.2)",
              }}
            >
              <BookOpen size={20} />
              Documentation Center
            </Link>
          </nav>

          <div style={{ marginTop: "auto", display: "flex", flexDirection: "column", gap: "12px" }}>
            <Link
              href="/docs"
              onClick={() => setMobileMenuOpen(false)}
              className="apple-btn apple-btn-primary"
              style={{ height: "48px", fontSize: "16px", width: "100%" }}
            >
              Get Started with NexusLB
              <ArrowUpRight size={18} />
            </Link>
            <a
              href="https://github.com/aazankhxn/NexusLB"
              target="_blank"
              rel="noreferrer"
              className="apple-btn apple-btn-secondary"
              style={{ height: "48px", fontSize: "16px", width: "100%" }}
            >
              <GithubIcon size={18} />
              View Source on GitHub
            </a>
          </div>
        </div>
      )}
    </>
  );
}
