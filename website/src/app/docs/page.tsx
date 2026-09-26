import { Navbar } from "@/components/Navbar";
import { DocsViewer } from "@/components/DocsViewer";
import Footer from "@/components/Footer";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Documentation — NexusLB",
  description: "Comprehensive guides, architecture overviews, routing algorithm mathematics, and production runbooks for NexusLB.",
};

export default function DocsPage() {
  return (
    <div style={{ minHeight: "100vh", position: "relative", overflowX: "hidden" }}>
      {/* Background Ambience */}
      <div
        style={{
          position: "fixed",
          top: "-200px",
          left: "50%",
          transform: "translateX(-50%)",
          width: "900px",
          height: "500px",
          background: "radial-gradient(circle, rgba(41, 151, 255, 0.09) 0%, transparent 75%)",
          filter: "blur(100px)",
          pointerEvents: "none",
          zIndex: 0,
        }}
      />

      <Navbar />

      <main style={{ position: "relative", zIndex: 1, paddingTop: "48px" }}>
        <DocsViewer />
      </main>

      <Footer />
    </div>
  );
}
