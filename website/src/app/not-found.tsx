import Link from "next/link";
import Image from "next/image";
import { ArrowLeft, Home, BookOpen } from "lucide-react";

export default function NotFound() {
  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        padding: "24px",
        textAlign: "center",
        background: "var(--bg-main)",
        color: "var(--text-primary)",
        position: "relative",
        overflow: "hidden",
      }}
    >
      {/* Background glow */}
      <div
        style={{
          position: "absolute",
          top: "20%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          width: "600px",
          height: "400px",
          background: "radial-gradient(circle, rgba(0, 113, 227, 0.15) 0%, transparent 70%)",
          filter: "blur(80px)",
          pointerEvents: "none",
        }}
      />

      <div style={{ position: "relative", zIndex: 1, maxWidth: "480px" }}>
        <div style={{ display: "flex", justifyContent: "center", marginBottom: "24px" }}>
          <Image
            src="/nexuslb.png"
            alt="NexusLB"
            width={72}
            height={72}
            style={{ borderRadius: "16px", objectFit: "contain" }}
          />
        </div>

        <div
          style={{
            fontSize: "12px",
            fontWeight: 700,
            textTransform: "uppercase",
            letterSpacing: "0.1em",
            color: "#2997ff",
            marginBottom: "8px",
          }}
        >
          404 &bull; Page Not Found
        </div>

        <h1
          style={{
            fontSize: "clamp(2rem, 4vw, 2.5rem)",
            fontWeight: 800,
            letterSpacing: "-0.03em",
            lineHeight: 1.15,
            color: "#ffffff",
            marginBottom: "16px",
          }}
        >
          The page you are looking for doesn’t exist.
        </h1>

        <p
          style={{
            fontSize: "14.5px",
            color: "var(--text-secondary)",
            lineHeight: 1.6,
            marginBottom: "32px",
          }}
        >
          The requested route could not be found on the server. You can return to the homepage or explore the documentation.
        </p>

        <div style={{ display: "flex", gap: "12px", justifyContent: "center", flexWrap: "wrap" }}>
          <Link
            href="/"
            className="apple-btn apple-btn-primary"
            style={{ display: "inline-flex", alignItems: "center", gap: "8px" }}
          >
            <Home size={15} />
            Back to Homepage
          </Link>
          <Link
            href="/docs"
            className="apple-btn apple-btn-secondary"
            style={{ display: "inline-flex", alignItems: "center", gap: "8px" }}
          >
            <BookOpen size={15} />
            Browse Documentation
          </Link>
        </div>
      </div>
    </div>
  );
}
