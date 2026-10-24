import { MoonIcon, SunIcon } from './PlaygroundIcons';

interface PlaygroundHeaderProps {
  dark: boolean;
  onToggleTheme: () => void;
  neon: string;
  neonDim: string;
  borderCol: string;
  textCol: string;
  mutedCol: string;
}

export function PlaygroundHeader({
  dark: D,
  onToggleTheme,
  neon,
  neonDim,
  borderCol,
  textCol,
  mutedCol,
}: PlaygroundHeaderProps) {
  return (
    <header
      style={{
        position: "relative",
        zIndex: 10,
        background: D ? "rgba(5,8,16,0.95)" : "rgba(255,255,255,0.95)",
        backdropFilter: "blur(16px)",
        borderBottom: `1px solid ${borderCol}`,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "12px 24px",
        flexShrink: 0,
      }}
    >
      {/* Logo */}
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <div
          style={{
            position: "relative",
            width: 38,
            height: 38,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            border: `1px solid ${neon}`,
            borderRadius: 8,
            background: neonDim,
            boxShadow: D
              ? `0 0 18px ${neon}35, inset 0 0 12px ${neon}08`
              : "none",
            transition: "all 0.4s",
          }}
        >
          <span style={{ fontSize: 18 }}>⚓</span>
          <div
            style={{
              position: "absolute",
              top: -1,
              left: -1,
              width: 8,
              height: 8,
              borderTop: `1.5px solid ${neon}`,
              borderLeft: `1.5px solid ${neon}`,
            }}
          />
          <div
            style={{
              position: "absolute",
              bottom: -1,
              right: -1,
              width: 8,
              height: 8,
              borderBottom: `1.5px solid ${neon}`,
              borderRight: `1.5px solid ${neon}`,
            }}
          />
        </div>
        <div>
          <div
            style={{
              fontSize: 13,
              fontWeight: 700,
              letterSpacing: "0.18em",
              color: neon,
              textShadow: D ? `0 0 22px ${neon}70` : "none",
              transition: "all 0.4s",
            }}
          >
            ANCHORKIT
          </div>
          <div
            style={{
              fontSize: 9,
              fontWeight: 600,
              letterSpacing: "0.15em",
              color: mutedCol,
              marginTop: 1,
            }}
          >
            SEP API PLAYGROUND
          </div>
        </div>
      </div>

      {/* Theme toggle */}
      <button
        onClick={onToggleTheme}
        aria-label={D ? "Switch to light mode" : "Switch to dark mode"}
        aria-pressed={D}
        title={D ? "Switch to light mode" : "Switch to dark mode"}
        style={{
          width: 36,
          height: 36,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          border: `1px solid ${borderCol}`,
          borderRadius: 8,
          background: D ? "rgba(0,0,0,0.3)" : "rgba(255,255,255,0.6)",
          color: textCol,
          cursor: "pointer",
          transition: "all 0.2s",
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.borderColor = neon;
          e.currentTarget.style.boxShadow = D
            ? `0 0 12px ${neon}40`
            : `0 0 8px ${neon}30`;
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.borderColor = borderCol;
          e.currentTarget.style.boxShadow = "none";
        }}
      >
        {D ? (
          <SunIcon aria-hidden="true" />
        ) : (
          <MoonIcon aria-hidden="true" />
        )}
      </button>
    </header>
  );
}
