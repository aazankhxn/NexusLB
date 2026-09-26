import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";
import { Showdown } from "@/components/Showdown";
import { AlgorithmSimulator } from "@/components/AlgorithmSimulator";
import { Features } from "@/components/Features";
import { ConfigBuilder } from "@/components/ConfigBuilder";
import { DocsViewer } from "@/components/DocsViewer";
import Footer from "@/components/Footer";

export default function Home() {
  return (
    <div style={{ minHeight: "100vh", position: "relative", overflowX: "hidden" }}>
      {/* Dynamic Background Glows (Apple Restrained Ambient Sheen) */}
      <div
        style={{
          position: "fixed",
          top: "-200px",
          left: "50%",
          transform: "translateX(-50%)",
          width: "min(900px, 100vw)",
          height: "600px",
          background: "radial-gradient(circle, rgba(41, 151, 255, 0.09) 0%, transparent 75%)",
          filter: "blur(100px)",
          pointerEvents: "none",
          zIndex: 0,
        }}
      />
      <div
        style={{
          position: "fixed",
          top: "800px",
          right: "-100px",
          width: "min(600px, 100vw)",
          height: "600px",
          background: "radial-gradient(circle, rgba(94, 92, 230, 0.05) 0%, transparent 70%)",
          filter: "blur(110px)",
          pointerEvents: "none",
          zIndex: 0,
        }}
      />

      {/* Navigation */}
      <Navbar />

      <main style={{ position: "relative", zIndex: 1 }}>
        {/* 1. Hero Section with Interactive macOS Terminal */}
        <Hero />

        {/* 2. Showdown Benchmarks vs Competitors */}
        <Showdown />

        {/* 3. Live Canvas Traffic Routing & Chaos Simulator */}
        <AlgorithmSimulator />

        {/* 4. Core Engineering Pillars (Bento Grid) */}
        <Features />

        {/* 5. Interactive Configuration Builder */}
        <ConfigBuilder />

        {/* 6. Built-in Apple-style Documentation Center */}
        <DocsViewer />
      </main>

      {/* 7. Apple-style System Specifications Footer */}
      <Footer />
    </div>
  );
}
