interface PlaygroundBackgroundProps {
  dark: boolean;
  neon: string;
}

export function PlaygroundBackground({ dark: D, neon }: PlaygroundBackgroundProps) {
  return (
    <>
      <style>{`
        @keyframes playground-scanline-sweep {
          from { top: -10%; }
          to { top: 110%; }
        }
        .playground-scanline-sweep { animation: playground-scanline-sweep 8s linear infinite; }
        @media (prefers-reduced-motion: reduce) {
          .playground-scanline-sweep { animation: none; }
        }
      `}</style>
      {/* Grid background */}
      <div
        style={{
          position: "fixed",
          inset: 0,
          pointerEvents: "none",
          backgroundImage: `linear-gradient(${D ? "rgba(80,120,255,0.04)" : "rgba(60,100,200,0.05)"} 1px, transparent 1px), linear-gradient(90deg, ${D ? "rgba(80,120,255,0.04)" : "rgba(60,100,200,0.05)"} 1px, transparent 1px)`,
          backgroundSize: "48px 48px",
        }}
      />

      {/* Ambient glows */}
      <div
        style={{
          position: "fixed",
          top: "-100px",
          left: "20%",
          width: 500,
          height: 500,
          borderRadius: "50%",
          background: neon,
          opacity: D ? 0.06 : 0.04,
          filter: "blur(120px)",
          pointerEvents: "none",
          transition: "background 0.4s",
        }}
      />
      <div
        style={{
          position: "fixed",
          bottom: "-100px",
          right: "15%",
          width: 380,
          height: 380,
          borderRadius: "50%",
          background: D ? "#4466ff" : "#2244cc",
          opacity: D ? 0.05 : 0.04,
          filter: "blur(100px)",
          pointerEvents: "none",
        }}
      />

      {/* Scanline (dark only) */}
      {D && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            pointerEvents: "none",
            zIndex: 50,
            overflow: "hidden",
          }}
        >
          <div
            style={{
              position: "absolute",
              inset: 0,
              backgroundImage:
                "repeating-linear-gradient(0deg,transparent,transparent 3px,rgba(0,0,0,0.18) 3px,rgba(0,0,0,0.18) 4px)",
              opacity: 0.4,
            }}
          />
          <div
            className="playground-scanline-sweep"
            style={{
              position: "absolute",
              left: 0,
              right: 0,
              height: 80,
              background: `linear-gradient(transparent,${neon}06,transparent)`,
            }}
          />
        </div>
      )}

      {/* Corner brackets (dark) */}
      {D && (
        <>
          <div
            style={{
              position: "fixed",
              top: 12,
              left: 12,
              width: 28,
              height: 28,
              borderTop: `1.5px solid ${neon}`,
              borderLeft: `1.5px solid ${neon}`,
              opacity: 0.4,
              pointerEvents: "none",
            }}
          />
          <div
            style={{
              position: "fixed",
              top: 12,
              right: 12,
              width: 28,
              height: 28,
              borderTop: `1.5px solid ${neon}`,
              borderRight: `1.5px solid ${neon}`,
              opacity: 0.4,
              pointerEvents: "none",
            }}
          />
          <div
            style={{
              position: "fixed",
              bottom: 12,
              left: 12,
              width: 28,
              height: 28,
              borderBottom: `1.5px solid ${neon}`,
              borderLeft: `1.5px solid ${neon}`,
              opacity: 0.4,
              pointerEvents: "none",
            }}
          />
          <div
            style={{
              position: "fixed",
              bottom: 12,
              right: 12,
              width: 28,
              height: 28,
              borderBottom: `1.5px solid ${neon}`,
              borderRight: `1.5px solid ${neon}`,
              opacity: 0.4,
              pointerEvents: "none",
            }}
          />
        </>
      )}
    </>
  );
}
