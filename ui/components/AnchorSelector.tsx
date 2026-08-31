import { useState, useEffect, useRef, useCallback } from "react";

export interface AnchorOption {
  id: string;
  name: string;
  endpoint?: string;
  healthScore: number; // 0–100
}

export interface AnchorSelectorProps {
  anchors: AnchorOption[];
  /** Pre-selected anchor id. If omitted the component auto-selects the highest-scoring anchor. */
  selectedId?: string;
  onChange: (anchor: AnchorOption) => void;
  /** Minimum health score an anchor must have to be considered auto-selectable (default: 0). */
  minHealthScore?: number;
  className?: string;
}

function scoreColor(score: number): string {
  if (score >= 80) return "#16a34a";
  if (score >= 60) return "#d97706";
  return "#dc2626";
}

function scoreLabel(score: number): string {
  if (score >= 80) return "Healthy";
  if (score >= 60) return "Degraded";
  return "Unhealthy";
}

export function AnchorSelector({
  anchors,
  selectedId,
  onChange,
  minHealthScore = 0,
  className,
}: AnchorSelectorProps) {
  const eligible = anchors.filter((a) => a.healthScore >= minHealthScore);
  const best = eligible.length > 0
    ? eligible.reduce((a, b) => (b.healthScore > a.healthScore ? b : a))
    : null;

  const [selected, setSelected] = useState<string | null>(
    selectedId ?? best?.id ?? null
  );
  const [focusedIndex, setFocusedIndex] = useState<number>(0);
  const userPicked = useRef(selectedId !== undefined);

  // Re-derive the effective selection whenever the caller controls selectedId, or
  // when the anchors list changes. While uncontrolled and not yet manually picked by
  // the user, this keeps tracking the current "best" anchor (e.g. as a live
  // health-score poll updates the anchors array) instead of freezing on the first
  // auto-selection. A manual pick is preserved as long as it stays eligible; if it
  // drops below minHealthScore it falls back to the new best.
  useEffect(() => {
    if (selectedId !== undefined) {
      setSelected(selectedId);
      return;
    }
    if (!userPicked.current) {
      setSelected(best?.id ?? null);
      return;
    }
    setSelected((prev) => {
      const prevStillEligible = prev != null && eligible.some((a) => a.id === prev);
      return prevStillEligible ? prev : best?.id ?? null;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [anchors, selectedId, minHealthScore]);

  // Notify parent when the selected anchor (or its data) changes
  useEffect(() => {
    if (selected == null) return;
    const anchor = anchors.find((a) => a.id === selected);
    if (anchor) onChange(anchor);
  }, [selected, anchors, onChange]);

  if (anchors.length === 0) {
    return (
      <div
        role="status"
        style={{ padding: "12px 16px", color: "var(--ak-text-muted)", fontSize: 13 }}
        className={className}
      >
        No anchors available.
      </div>
    );
  }

  return (
    <div
      role="listbox"
      aria-label="Select anchor"
      className={className}
      style={{ display: "flex", flexDirection: "column", gap: 8 }}
    >
      {anchors.map((anchor, index) => {
        const isSelected = anchor.id === selected;
        const isDisabled = anchor.healthScore < minHealthScore;
        const isFocused = index === focusedIndex;
        const color = scoreColor(anchor.healthScore);

        const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
          if (e.key === "ArrowDown") {
            e.preventDefault();
            let nextIndex = index + 1;
            while (nextIndex < anchors.length && anchors[nextIndex].healthScore < minHealthScore) {
              nextIndex++;
            }
            if (nextIndex < anchors.length) {
              setFocusedIndex(nextIndex);
            }
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            let prevIndex = index - 1;
            while (prevIndex >= 0 && anchors[prevIndex].healthScore < minHealthScore) {
              prevIndex--;
            }
            if (prevIndex >= 0) {
              setFocusedIndex(prevIndex);
            }
          } else if (e.key === "Home") {
            e.preventDefault();
            let firstIndex = 0;
            while (firstIndex < anchors.length && anchors[firstIndex].healthScore < minHealthScore) {
              firstIndex++;
            }
            if (firstIndex < anchors.length) {
              setFocusedIndex(firstIndex);
            }
          } else if (e.key === "End") {
            e.preventDefault();
            let lastIndex = anchors.length - 1;
            while (lastIndex >= 0 && anchors[lastIndex].healthScore < minHealthScore) {
              lastIndex--;
            }
            if (lastIndex >= 0) {
              setFocusedIndex(lastIndex);
            }
          } else if (!isDisabled && (e.key === "Enter" || e.key === " ")) {
            e.preventDefault();
            userPicked.current = true;
            setSelected(anchor.id);
          }
        };

        return (
          <div
            key={anchor.id}
            role="option"
            aria-selected={isSelected}
            aria-disabled={isDisabled}
            tabIndex={isFocused && !isDisabled ? 0 : -1}
            onClick={() => {
              if (!isDisabled) {
                userPicked.current = true;
                setSelected(anchor.id);
                setFocusedIndex(index);
              }
            }}
            onKeyDown={handleKeyDown}
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "10px 14px",
              borderRadius: 10,
              border: `2px solid ${isSelected ? color : "var(--ak-border)"}`,
              background: isSelected ? `${color}12` : "var(--ak-surface)",
              cursor: isDisabled ? "not-allowed" : "pointer",
              opacity: isDisabled ? 0.5 : 1,
              transition: "border-color 0.15s, background 0.15s",
              outline: "none",
            }}
          >
            {/* Left: name + endpoint */}
            <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
              <span style={{ fontWeight: 600, fontSize: 14, color: "var(--ak-text)" }}>
                {anchor.name}
                {anchor.id === best?.id && (
                  <span
                    style={{
                      marginLeft: 8,
                      fontSize: 10,
                      fontWeight: 700,
                      letterSpacing: "0.08em",
                      textTransform: "uppercase",
                      padding: "1px 6px",
                      borderRadius: 8,
                      background: "var(--ak-status-completed-bg)",
                      color: "var(--ak-status-completed-color)",
                    }}
                  >
                    Best
                  </span>
                )}
              </span>
              {anchor.endpoint && (
                <span style={{ fontSize: 11, color: "var(--ak-text-muted)", fontFamily: "monospace" }}>
                  {anchor.endpoint}
                </span>
              )}
            </div>

            {/* Right: health score badge */}
            <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 2 }}>
              <span style={{ fontSize: 18, fontWeight: 700, color, lineHeight: 1 }}>
                {anchor.healthScore}
              </span>
              <span style={{ fontSize: 10, fontWeight: 600, color, textTransform: "uppercase", letterSpacing: "0.06em" }}>
                {scoreLabel(anchor.healthScore)}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
