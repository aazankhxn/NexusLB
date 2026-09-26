"use client";

import { useEffect, useRef, useState } from "react";
import { Activity, AlertTriangle, CheckCircle2, Sliders } from "lucide-react";

export function AlgorithmSimulator() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [algorithm, setAlgorithm] = useState<"adaptive" | "p2c" | "least_connections" | "round_robin">("adaptive");
  const [isNode3Degraded, setIsNode3Degraded] = useState(false);

  const [nodeData, setNodeData] = useState([
    { id: 1, name: "node-01:8080", conns: 12, lat: 45, reqs: 0 },
    { id: 2, name: "node-02:8080", conns: 14, lat: 50, reqs: 0 },
    { id: 3, name: "node-03:8080", conns: 15, lat: 48, reqs: 0 },
    { id: 4, name: "node-04:8080", conns: 13, lat: 46, reqs: 0 },
  ]);

  const stateRef = useRef({
    algorithm: "adaptive",
    isNode3Degraded: false,
    nodes: [
      { conns: 12, lat: 45, reqs: 0 },
      { conns: 14, lat: 50, reqs: 0 },
      { conns: 15, lat: 48, reqs: 0 },
      { conns: 13, lat: 46, reqs: 0 },
    ],
    packets: [] as Array<{
      x: number;
      y: number;
      targetX: number;
      targetY: number;
      progress: number;
      targetIdx: number;
      color: string;
    }>,
    rrIndex: 0,
  });

  useEffect(() => {
    stateRef.current.algorithm = algorithm;
    stateRef.current.isNode3Degraded = isNode3Degraded;
    stateRef.current.nodes[2].lat = isNode3Degraded ? 950 : 48;
  }, [algorithm, isNode3Degraded]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animId: number;

    const resize = () => {
      if (canvas.parentElement) {
        canvas.width = canvas.parentElement.clientWidth;
        canvas.height = 180;
      }
    };
    resize();
    window.addEventListener("resize", resize);

    // Spawn packets
    const spawnTimer = setInterval(() => {
      const state = stateRef.current;
      if (state.packets.length < 32) {
        let targetIdx = 0;
        switch (state.algorithm) {
          case "round_robin":
            targetIdx = state.rrIndex % 4;
            state.rrIndex++;
            break;
          case "least_connections": {
            let minC = Infinity;
            state.nodes.forEach((n, idx) => {
              if (n.conns < minC) {
                minC = n.conns;
                targetIdx = idx;
              }
            });
            break;
          }
          case "p2c": {
            const i1 = Math.floor(Math.random() * 4);
            let i2 = Math.floor(Math.random() * 4);
            if (i1 === i2) i2 = (i1 + 1) % 4;
            const s1 = state.nodes[i1].lat * (1 + state.nodes[i1].conns);
            const s2 = state.nodes[i2].lat * (1 + state.nodes[i2].conns);
            targetIdx = s1 <= s2 ? i1 : i2;
            break;
          }
          case "adaptive":
          default: {
            let minScore = Infinity;
            state.nodes.forEach((n, idx) => {
              const score = n.lat * (1 + n.conns);
              if (score < minScore) {
                minScore = score;
                targetIdx = idx;
              }
            });
            break;
          }
        }

        state.nodes[targetIdx].reqs++;
        state.nodes[targetIdx].conns = Math.min(state.nodes[targetIdx].conns + 1, 95);

        setTimeout(() => {
          state.nodes[targetIdx].conns = Math.max(state.nodes[targetIdx].conns - 1, 8);
        }, state.isNode3Degraded && targetIdx === 2 ? 800 : 250);

        const targetX = ((targetIdx + 0.5) / 4) * canvas.width;
        state.packets.push({
          x: canvas.width / 2,
          y: 8,
          targetX,
          targetY: canvas.height - 8,
          progress: 0,
          targetIdx,
          color: targetIdx === 2 && state.isNode3Degraded ? "#ff453a" : "#00f0ff",
        });
      }
    }, 110);

    // Update DOM sync interval
    const syncTimer = setInterval(() => {
      const state = stateRef.current;
      setNodeData([
        { id: 1, name: "node-01:8080", conns: state.nodes[0].conns, lat: 45, reqs: state.nodes[0].reqs },
        { id: 2, name: "node-02:8080", conns: state.nodes[1].conns, lat: 50, reqs: state.nodes[1].reqs },
        { id: 3, name: "node-03:8080", conns: state.nodes[2].conns, lat: state.isNode3Degraded ? 950 : 48, reqs: state.nodes[2].reqs },
        { id: 4, name: "node-04:8080", conns: state.nodes[3].conns, lat: 46, reqs: state.nodes[3].reqs },
      ]);
    }, 200);

    // Animation loop
    const render = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      const state = stateRef.current;

      // Draw connection lines
      for (let i = 0; i < 4; i++) {
        const targetX = ((i + 0.5) / 4) * canvas.width;
        ctx.strokeStyle = i === 2 && state.isNode3Degraded
          ? "rgba(255, 69, 58, 0.25)"
          : "rgba(255, 255, 255, 0.08)";
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(canvas.width / 2, 8);
        ctx.lineTo(targetX, canvas.height - 8);
        ctx.stroke();
      }

      // Draw packets
      for (let i = state.packets.length - 1; i >= 0; i--) {
        const p = state.packets[i];
        p.progress += 0.035;

        const curX = p.x + (p.targetX - p.x) * p.progress;
        const curY = p.y + (p.targetY - p.y) * p.progress;

        ctx.fillStyle = p.color;
        ctx.shadowBlur = 8;
        ctx.shadowColor = p.color;
        ctx.beginPath();
        ctx.arc(curX, curY, 4, 0, Math.PI * 2);
        ctx.fill();
        ctx.shadowBlur = 0;

        if (p.progress >= 1) {
          state.packets.splice(i, 1);
        }
      }

      animId = requestAnimationFrame(render);
    };
    render();

    return () => {
      clearInterval(spawnTimer);
      clearInterval(syncTimer);
      cancelAnimationFrame(animId);
      window.removeEventListener("resize", resize);
    };
  }, []);

  return (
    <section id="simulator" className="section" style={{ position: "relative" }}>
      <div className="container">
        <div className="section-label">Interactive Simulation</div>
        <h2 className="section-title">Dynamic Scheduling in Action</h2>
        <p className="section-desc">
          Watch how NexusLB adaptively steers traffic around degraded backends in real time compared to blind round-robin proxies.
        </p>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "28px", maxWidth: "1080px", margin: "0 auto" }}>
          {/* Controls Card */}
          <div className="apple-card" style={{ display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "18px" }}>
                <Sliders size={18} color="var(--accent-cyan)" />
                <span style={{ fontSize: "14px", fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em", color: "#ffffff" }}>
                  Algorithm Selector
                </span>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "8px", marginBottom: "28px" }}>
                <button
                  className={`apple-segment-btn ${algorithm === "adaptive" ? "active" : ""}`}
                  style={{ textAlign: "left", padding: "12px 18px", display: "flex", justifyContent: "space-between", alignItems: "center" }}
                  onClick={() => setAlgorithm("adaptive")}
                >
                  <span>Adaptive Scoring</span>
                  <span style={{ fontSize: "11px", color: "var(--accent-cyan)", fontFamily: "var(--font-mono)" }}>Latency+Load</span>
                </button>
                <button
                  className={`apple-segment-btn ${algorithm === "p2c" ? "active" : ""}`}
                  style={{ textAlign: "left", padding: "12px 18px", display: "flex", justifyContent: "space-between", alignItems: "center" }}
                  onClick={() => setAlgorithm("p2c")}
                >
                  <span>Power of Two Choices (P2C)</span>
                  <span style={{ fontSize: "11px", color: "var(--text-tertiary)", fontFamily: "var(--font-mono)" }}>O(1)</span>
                </button>
                <button
                  className={`apple-segment-btn ${algorithm === "least_connections" ? "active" : ""}`}
                  style={{ textAlign: "left", padding: "12px 18px", display: "flex", justifyContent: "space-between", alignItems: "center" }}
                  onClick={() => setAlgorithm("least_connections")}
                >
                  <span>Least Connections</span>
                  <span style={{ fontSize: "11px", color: "var(--text-tertiary)", fontFamily: "var(--font-mono)" }}>Standard</span>
                </button>
                <button
                  className={`apple-segment-btn ${algorithm === "round_robin" ? "active" : ""}`}
                  style={{ textAlign: "left", padding: "12px 18px", display: "flex", justifyContent: "space-between", alignItems: "center" }}
                  onClick={() => setAlgorithm("round_robin")}
                >
                  <span>Strict Round Robin</span>
                  <span style={{ fontSize: "11px", color: "var(--accent-rose)", fontFamily: "var(--font-mono)" }}>Blind</span>
                </button>
              </div>
            </div>

            {/* Chaos Injection */}
            <div style={{ padding: "18px", borderRadius: "var(--radius-md)", backgroundColor: "rgba(255, 255, 255, 0.03)", border: "1px solid rgba(255, 255, 255, 0.08)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "8px" }}>
                <AlertTriangle size={16} color={isNode3Degraded ? "var(--accent-rose)" : "var(--accent-amber)"} />
                <span style={{ fontSize: "13px", fontWeight: 600, color: "#ffffff" }}>Chaos Injection</span>
              </div>
              <p style={{ fontSize: "12px", color: "var(--text-secondary)", marginBottom: "14px" }}>
                Inject a 950ms GC spike on Node 3. Notice how Adaptive routing stops sending traffic to Node 3, whereas Round Robin keeps sending 25% of requests to it!
              </p>
              <button
                className={`apple-btn apple-btn-sm ${isNode3Degraded ? "apple-btn-primary" : "apple-btn-secondary"}`}
                style={{ width: "100%", justifyContent: "center" }}
                onClick={() => setIsNode3Degraded(!isNode3Degraded)}
              >
                {isNode3Degraded ? "Heal Node 3 (Clear GC Spike)" : "Simulate 950ms GC Pause on Node 3"}
              </button>
            </div>
          </div>

          {/* Visual Canvas Stage */}
          <div className="apple-card" style={{ display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
            {/* Client Ingress Pill */}
            <div style={{ textAlign: "center", marginBottom: "16px" }}>
              <div
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "8px",
                  padding: "8px 20px",
                  borderRadius: "9999px",
                  background: "linear-gradient(135deg, #00f0ff, #0071e3)",
                  color: "#000000",
                  fontWeight: 700,
                  fontSize: "13px",
                  boxShadow: "0 0 20px rgba(0, 240, 255, 0.35)",
                }}
              >
                <Activity size={14} />
                Client Ingress Stream (120 req/s)
              </div>
            </div>

            {/* Canvas */}
            <canvas ref={canvasRef} style={{ width: "100%", height: "180px", display: "block" }} />

            {/* 4 Node Cluster Cards */}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))", gap: "10px", marginTop: "16px" }}>
              {nodeData.map((node) => {
                const isDegraded = node.id === 3 && isNode3Degraded;
                return (
                  <div
                    key={node.id}
                    style={{
                      padding: "14px 10px",
                      borderRadius: "var(--radius-sm)",
                      backgroundColor: isDegraded ? "rgba(255, 69, 58, 0.12)" : "rgba(255, 255, 255, 0.04)",
                      border: `1px solid ${isDegraded ? "rgba(255, 69, 58, 0.5)" : "rgba(255, 255, 255, 0.08)"}`,
                      textAlign: "center",
                      transition: "all 0.3s var(--spring-snappy)",
                    }}
                  >
                    <div style={{ fontSize: "11px", fontWeight: 700, fontFamily: "var(--font-mono)", color: "#ffffff", marginBottom: "4px" }}>
                      node-0{node.id}
                    </div>
                    <span
                      style={{
                        display: "inline-block",
                        fontSize: "9px",
                        fontWeight: 700,
                        padding: "2px 6px",
                        borderRadius: "9999px",
                        backgroundColor: isDegraded ? "rgba(255, 69, 58, 0.2)" : "rgba(48, 209, 88, 0.15)",
                        color: isDegraded ? "var(--accent-rose)" : "var(--accent-emerald)",
                        marginBottom: "8px",
                      }}
                    >
                      {isDegraded ? "SLOW" : "UP"}
                    </span>
                    <div style={{ fontSize: "11px", color: "var(--text-tertiary)" }}>
                      Lat: <span style={{ color: isDegraded ? "var(--accent-rose)" : "#ffffff", fontFamily: "var(--font-mono)", fontWeight: 600 }}>{node.lat} {isDegraded ? "ms" : "µs"}</span>
                    </div>
                    <div style={{ fontSize: "11px", color: "var(--text-tertiary)" }}>
                      Conns: <span style={{ color: "#ffffff", fontFamily: "var(--font-mono)" }}>{node.conns}</span>
                    </div>
                    <div style={{ fontSize: "11px", color: "var(--text-tertiary)" }}>
                      Reqs: <span style={{ color: "var(--accent-cyan)", fontFamily: "var(--font-mono)", fontWeight: 600 }}>{node.reqs}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
