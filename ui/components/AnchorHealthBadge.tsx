import React, { useState } from "react";

export interface AnchorHealthBadgeProps {
  /** Health score 0–100 */
  score: number;
  /** Show numeric score inside the badge */
  showScore?: boolean;
  /** Optional tooltip text override */
  tooltip?: string;
  className?: string;
}

function getColor(score: number): { color: string; bg: string; border: string; label: string } {
  if (score >= 80) return { color: "#059669", bg: "rgba(5,150,105,0.12)", border: "rgba(5,150,105,0.35)", label: "Healthy" };
  if (score >= 60) return { color: "#d97706", bg: "rgba(217,119,6,0.12)", border: "rgba(217,119,6,0.35)", label: "Degraded" };
  return { color: "#dc2626", bg: "rgba(220,38,38,0.12)", border: "rgba(220,38,38,0.35)", label: "Poor" };
}

export function AnchorHealthBadge({
  score,
  showScore = true,
  tooltip,
  className,
}: AnchorHealthBadgeProps) {
  const [hovered, setHovered] = useState(false);
  const clamped = Math.max(0, Math.min(100, Math.round(score)));
  const { color, bg, border, label } = getColor(clamped);
  const tooltipText = tooltip ?? `Health score: ${clamped}/100 — ${label}`;

  return (
    <span
      className={className}
      role="status"
      aria-label={tooltipText}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        position: "relative",
        display: "inline-flex",
        alignItems: "center",
        gap: 5,
        padding: "3px 9px",
        borderRadius: 20,
        background: bg,
        border: `1px solid ${border}`,
        color,
        fontFamily: "'Sora', sans-serif",
        fontSize: 11,
        fontWeight: 700,
        cursor: "default",
        userSelect: "none",
        whiteSpace: "nowrap",
      }}
    >
      <span
        style={{
          width: 7,
          height: 7,
          borderRadius: "50%",
          background: color,
          flexShrink: 0,
          boxShadow: `0 0 6px ${color}`,
        }}
      />
      {label}
      {showScore && (
        <span style={{ opacity: 0.8, fontFamily: "monospace" }}>{clamped}</span>
      )}

      {hovered && (
        <span
          role="tooltip"
          style={{
            position: "absolute",
            bottom: "calc(100% + 6px)",
            left: "50%",
            transform: "translateX(-50%)",
            background: "var(--ak-surface, #1c1916)",
            border: `1px solid ${border}`,
            color: "var(--ak-text, #e7e5e0)",
            borderRadius: 7,
            padding: "5px 10px",
            fontSize: 11,
            whiteSpace: "nowrap",
            pointerEvents: "none",
            zIndex: 9999,
            boxShadow: "0 4px 16px rgba(0,0,0,0.3)",
          }}
        >
          {tooltipText}
        </span>
      )}
    </span>
  );
}
