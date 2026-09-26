"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import { ShieldCheck, X, ArrowRight } from "lucide-react";

export function PrivacyBanner() {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const dismissed = localStorage.getItem("nexuslb_privacy_pill_dismissed");
    if (!dismissed) {
      // Delay slightly for fluid Apple entrance transition
      const timer = setTimeout(() => setVisible(true), 1200);
      return () => clearTimeout(timer);
    }
  }, []);

  const dismiss = () => {
    localStorage.setItem("nexuslb_privacy_pill_dismissed", "true");
    setVisible(false);
  };

  if (!visible) return null;

  return (
    <aside
      aria-label="Privacy notice"
      style={{
        position: "fixed",
        bottom: "24px",
        left: "50%",
        transform: "translateX(-50%)",
        zIndex: 999,
        maxWidth: "92vw",
        width: "560px",
        animation: "slideUp 0.4s cubic-bezier(0.16, 1, 0.3, 1)",
      }}
    >
      <div
        style={{
          background: "rgba(22, 24, 34, 0.85)",
          backdropFilter: "blur(24px) saturate(180%)",
          WebkitBackdropFilter: "blur(24px) saturate(180%)",
          border: "1px solid rgba(255, 255, 255, 0.12)",
          borderRadius: "100px",
          padding: "10px 14px 10px 18px",
          boxShadow: "0 20px 40px -10px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.05)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "14px",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "12px", minWidth: 0 }}>
          <div
            style={{
              width: "28px",
              height: "28px",
              borderRadius: "50%",
              background: "rgba(48, 209, 88, 0.15)",
              border: "1px solid rgba(48, 209, 88, 0.3)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "#30d158",
              flexShrink: 0,
            }}
          >
            <ShieldCheck size={16} />
          </div>
          <p
            style={{
              fontSize: "13px",
              color: "var(--text-secondary)",
              margin: 0,
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            <strong style={{ color: "#ffffff", fontWeight: 600 }}>Zero Telemetry by Design.</strong> We do not track, collect, or store your data.
          </p>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "8px", flexShrink: 0 }}>
          <Link
            href="/privacy"
            style={{
              fontSize: "12px",
              fontWeight: 600,
              color: "#2997ff",
              textDecoration: "none",
              display: "inline-flex",
              alignItems: "center",
              gap: "4px",
              padding: "4px 8px",
              borderRadius: "6px",
              transition: "color 0.2s ease",
            }}
          >
            Learn More
            <ArrowRight size={12} />
          </Link>
          <button
            onClick={dismiss}
            aria-label="Dismiss privacy notification"
            style={{
              background: "rgba(255, 255, 255, 0.08)",
              border: "none",
              color: "var(--text-muted)",
              width: "24px",
              height: "24px",
              borderRadius: "50%",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              cursor: "pointer",
              transition: "all 0.15s ease",
            }}
            onMouseEnter={(e) => (e.currentTarget.style.color = "#ffffff")}
            onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-muted)")}
          >
            <X size={13} />
          </button>
        </div>
      </div>
    </aside>
  );
}
