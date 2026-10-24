import { useState, useEffect, useRef } from "react";

import './themes.css';

// ─── Types ────────────────────────────────────────────────────────────────────

export type TxStatus = "initiated" | "awaiting_user" | "pending" | "processing" | "completed" | "failed" | "refunded";
export type TxType   = "deposit" | "withdrawal";

export interface TxEvent {
  status: TxStatus;
  label?: string;
  description?: string;
  timestamp?: string;         // ISO string or readable label
  txHash?: string;            // on-chain hash, shown on completed
  detail?: string;            // any extra note
  rawApiResponse?: unknown;   // raw anchor API response, shown in expanded view
  errorMessage?: string;      // error detail for this step, shown in expanded view
}

export interface TransactionTimelineProps {
  type: TxType;
  amount: string;
  asset: string;
  id?: string;
  events: TxEvent[];
  currentStatus: TxStatus;
  estimatedCompletionAt?: number; // timestamp in ms
  onRetry?: () => void;
  onClose?: () => void;
  className?: string;
}

// ─── Constants ────────────────────────────────────────────────────────────────

const STATUS_ORDER: TxStatus[] = ["initiated", "awaiting_user", "pending", "processing", "completed"];

const STATUS_META: Record<TxStatus, { label: string; color: string; bg: string; border: string; icon: string }> = {
  initiated:     { label: "Initiated",      color: "var(--ak-status-initiated-color)",  bg: "var(--ak-status-initiated-bg)",  border: "var(--ak-status-initiated-border)",  icon: "◎"  },
  awaiting_user: { label: "Action Required", color: "var(--ak-status-awaiting-user-color)",  bg: "var(--ak-status-awaiting-user-bg)",  border: "var(--ak-status-awaiting-user-border)",  icon: "⚠"  },
  pending:       { label: "Pending",         color: "var(--ak-status-pending-color)",    bg: "var(--ak-status-pending-bg)",    border: "var(--ak-status-pending-border)",    icon: "◌"  },
  processing:    { label: "Processing",      color: "var(--ak-status-processing-color)", bg: "var(--ak-status-processing-bg)", border: "var(--ak-status-processing-border)", icon: "◈"  },
  completed:     { label: "Completed",       color: "var(--ak-status-completed-color)",  bg: "var(--ak-status-completed-bg)",  border: "var(--ak-status-completed-border)",  icon: "✓"  },
  failed:        { label: "Failed",          color: "var(--ak-status-failed-color)",     bg: "var(--ak-status-failed-bg)",     border: "var(--ak-status-failed-border)",     icon: "✕"  },
  refunded:      { label: "Refunded",        color: "var(--ak-status-refunded-color)",       bg: "var(--ak-status-refunded-bg)",       border: "var(--ak-status-refunded-border)",       icon: "↩"  },
};

const DEFAULT_DESCRIPTIONS: Record<TxStatus, Record<TxType, string>> = {
  initiated:     { deposit: "Deposit request received by anchor.",                withdrawal: "Withdrawal request received by anchor."               },
  awaiting_user: { deposit: "Please initiate your transfer to the anchor.",       withdrawal: "Your confirmation is required to continue."           },
  pending:       { deposit: "Awaiting your funds on the external rail.",          withdrawal: "Awaiting Stellar transaction confirmation."           },
  processing:    { deposit: "Funds received — minting assets on-chain.",          withdrawal: "Processing payment to your bank account."            },
  completed:     { deposit: "Assets delivered to your Stellar account.",          withdrawal: "Funds sent to your destination account."             },
  failed:        { deposit: "Deposit could not be completed.",                    withdrawal: "Withdrawal could not be completed."                  },
  refunded:      { deposit: "Deposit could not be processed — funds refunded.",   withdrawal: "Withdrawal reversed — funds returned to your wallet." },
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

function isFailed(status: TxStatus): boolean { return status === "failed"; }
function isCompleted(status: TxStatus): boolean { return status === "completed"; }
function isRefunded(status: TxStatus): boolean { return status === "refunded"; }
function getOrderIndex(s: TxStatus): number { return STATUS_ORDER.indexOf(s); }

function truncateHash(h: string): string {
  return h.length > 16 ? `${h.slice(0, 8)}…${h.slice(-8)}` : h;
}

function formatTs(ts?: string): string {
  if (!ts) return "";
  const d = new Date(ts);
  if (isNaN(d.getTime())) return ts;
  return d.toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

// ─── Inline styles helpers ────────────────────────────────────────────────────

const mono: React.CSSProperties = { fontFamily: "'IBM Plex Mono', 'Fira Code', monospace" };
const serif: React.CSSProperties = { fontFamily: "'DM Serif Display', 'Georgia', serif" };
const sans: React.CSSProperties  = { fontFamily: "'DM Sans', 'Helvetica Neue', sans-serif" };

// ─── Sub-components ───────────────────────────────────────────────────────────

function NodeIcon({
  status, active, done, failed,
}: { status: TxStatus; active: boolean; done: boolean; failed: boolean }) {
  const m = STATUS_META[status];
  const isPending = active && status === "pending";
  const isProcessing = active && status === "processing";

  return (
    <div style={{ position: "relative", width: 40, height: 40, flexShrink: 0 }}>
      {/* Outer pulse for active */}
      {active && !failed && (
        <div style={{
          position: "absolute", inset: -5, borderRadius: "50%",
          border: `1.5px solid ${m.color}`,
          opacity: 0,
          animation: "txs-ping 2s ease-out infinite",
        }} />
      )}
      {/* Spinning ring for processing */}
      {isProcessing && (
        <div style={{
          position: "absolute", inset: -3, borderRadius: "50%",
          border: `1.5px solid transparent`,
          borderTopColor: m.color,
          animation: "txs-spin 1.2s linear infinite",
        }} />
      )}
      {/* Node circle */}
      <div style={{
        width: 40, height: 40, borderRadius: "50%",
        display: "flex", alignItems: "center", justifyContent: "center",
        border: `2px solid ${done || active ? m.color : "var(--ak-connector-bg)"}`,
        background: done || active ? m.bg : "var(--ak-surface-2)",
        boxShadow: active || done ? `0 0 0 4px ${m.bg}, 0 2px 12px ${m.color}28` : "none",
        transition: "all 0.5s cubic-bezier(0.4,0,0.2,1)",
        position: "relative", zIndex: 1,
        fontSize: done && !active ? 16 : 14,
        color: done || active ? m.color : "var(--ak-text-subtle)",
        fontWeight: 700,
      }}>
        {/* Spin indicator dots for pending */}
        {isPending ? (
          <span style={{ display: "flex", gap: 2 }}>
            {[0, 1, 2].map(i => (
              <span key={i} style={{
                width: 4, height: 4, borderRadius: "50%",
                background: m.color,
                animation: `txs-dot-pulse 1.2s ease-in-out ${i * 0.2}s infinite`,
                display: "inline-block",
              }} />
            ))}
          </span>
        ) : (
          <span style={{ lineHeight: 1, filter: (done || active) ? `drop-shadow(0 0 4px ${m.color}80)` : "none" }}>
            {m.icon}
          </span>
        )}
      </div>
    </div>
  );
}

function ProgressBar({
  from, to, active, done,
}: { from: string; to: string; active: boolean; done: boolean }) {
  return (
    <div style={{
      position: "absolute", left: 19, top: 40, width: 2, height: "calc(100% - 40px)",
      background: "var(--ak-connector-bg)", borderRadius: 2, overflow: "hidden",
    }}>
      <div style={{
        position: "absolute", top: 0, left: 0, width: "100%",
        height: done ? "100%" : active ? "50%" : "0%",
        background: `linear-gradient(to bottom, ${from}, ${to})`,
        transition: "height 0.8s cubic-bezier(0.4,0,0.2,1)",
        borderRadius: 2,
      }} />
      {active && (
        <div style={{
          position: "absolute", top: active ? "45%" : "95%", left: "50%",
          transform: "translate(-50%,-50%)",
          width: 8, height: 8, borderRadius: "50%",
          background: to, opacity: 0.6,
          animation: "txs-drip 1.6s ease-in-out infinite",
        }} />
      )}
    </div>
  );
}

// ─── Main Component ───────────────────────────────────────────────────────────

export function TransactionTimeline({
  type, amount, asset, id, events, currentStatus, estimatedCompletionAt, onRetry, onClose,
}: TransactionTimelineProps) {
  const failed = isFailed(currentStatus);
  const completed = isCompleted(currentStatus);
  const refunded = isRefunded(currentStatus);
  const currentIndex = getOrderIndex(currentStatus);

  const [timeLeft, setTimeLeft] = useState<number | null>(null);
  const [expandedStep, setExpandedStep] = useState<TxStatus | null>(null);

  const toggleStep = (status: TxStatus) => {
    setExpandedStep(prev => (prev === status ? null : status));
  };

  useEffect(() => {
    if (!estimatedCompletionAt || completed || failed) {
      setTimeLeft(null);
      return;
    }

    const update = () => {
      const now = Date.now();
      const diff = Math.max(0, Math.floor((estimatedCompletionAt - now) / 1000));
      setTimeLeft(diff);
      return diff;
    };

    update();
    const interval = setInterval(() => {
      const diff = update();
      if (diff === 0) clearInterval(interval);
    }, 1000);

    return () => clearInterval(interval);
  }, [estimatedCompletionAt, completed, failed]);

  // Build display steps — always show the 4 standard steps,
  // replace "completed" with "failed" node if tx failed
  const steps = STATUS_ORDER.map((s) => {
    const event = events.find(e => e.status === s);
    const stepIndex = getOrderIndex(s);
    const isDone = failed
      ? stepIndex < currentIndex
      : stepIndex <= currentIndex;
    const isActive = s === currentStatus && !failed;
    const m = STATUS_META[s];

    return { status: s, event, isDone, isActive, m, stepIndex };
  });

  const statusMeta = STATUS_META[currentStatus];

  return (
    <div style={{
      ...sans,
      background: "var(--ak-surface-3)",
      borderRadius: 20,
      border: "1px solid var(--ak-border)",
      boxShadow: "var(--ak-shadow-md)",
      overflow: "hidden",
      maxWidth: 420,
      width: "100%",
    }}>

      {/* ── Header ── */}
      <div style={{
        padding: "22px 24px 20px",
        background: failed ? "var(--ak-status-failed-bg)" : completed ? "var(--ak-status-completed-bg)" : "var(--ak-status-initiated-bg)",
        borderBottom: `1px solid ${failed ? "var(--ak-status-failed-border)" : completed ? "var(--ak-status-completed-border)" : "var(--ak-status-initiated-border)"}`,
        display: "flex", flexDirection: "column", gap: 14,
      }}>
        {/* Type pill + ID */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <span style={{
            ...mono, fontSize: 10, fontWeight: 600,
            letterSpacing: "0.12em", textTransform: "uppercase",
            padding: "3px 10px", borderRadius: 20,
            background: type === "deposit" ? "var(--ak-method-get-bg)" : "var(--ak-status-failed-bg)",
            color: type === "deposit" ? "var(--ak-method-get-color)" : "var(--ak-status-failed-color)",
            border: `1px solid ${type === "deposit" ? "var(--ak-status-processing-border)" : "var(--ak-status-failed-border)"}`,
          }}>
            {type === "deposit" ? "↓ Deposit" : "↑ Withdrawal"}
          </span>
          {id && (
            <span style={{ ...mono, fontSize: 10, color: "var(--ak-text-muted)", letterSpacing: "0.06em" }}>
              #{id.slice(-8)}
            </span>
          )}
        </div>

        {/* Amount */}
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <span style={{ ...serif, fontSize: 36, fontWeight: 400, color: failed ? "var(--ak-status-failed-color)" : completed ? "var(--ak-status-completed-color)" : "var(--ak-text)", lineHeight: 1 }}>
            {amount}
          </span>
          <span style={{ ...mono, fontSize: 14, color: "var(--ak-text-muted)", fontWeight: 500 }}>{asset}</span>
        </div>

        {/* Status badge */}
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <div style={{
            width: 8, height: 8, borderRadius: "50%",
            background: statusMeta.color,
            boxShadow: `0 0 0 2px ${statusMeta.bg}`,
            animation: (!failed && !completed && !refunded) ? "txs-ping-slow 2.5s ease-in-out infinite" : "none",
            flexShrink: 0,
          }} />
          <span style={{ ...mono, fontSize: 11, fontWeight: 600, color: statusMeta.color, letterSpacing: "0.1em", textTransform: "uppercase" }}>
            {statusMeta.label}
          </span>
        </div>

        {/* Countdown */}
        {timeLeft !== null && (
          <div style={{
            display: "flex", alignItems: "center", gap: 8,
            marginTop: 4, padding: "8px 12px", borderRadius: 10,
            background: "rgba(0,0,0,0.04)", width: "fit-content",
            border: "1px solid rgba(0,0,0,0.05)"
          }}>
            <span style={{ fontSize: 10, color: "#64748b", fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em" }}>Est. Completion</span>
            <div style={{ width: 1, height: 12, background: "#cbd5e1" }} />
            <span style={{ ...mono, fontSize: 12, color: timeLeft < 60 ? "#dc2626" : "#334155", fontWeight: 700 }}>
              {Math.floor(timeLeft / 60)}:{(timeLeft % 60).toString().padStart(2, "0")}
            </span>
          </div>
        )}
      </div>

      {/* ── Timeline ── */}
      <div style={{ padding: "24px 24px 20px" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
          {steps.map((step, i) => {
            const isLast = i === steps.length - 1;
            const nextMeta = !isLast ? STATUS_META[steps[i + 1].status] : null;
            const desc = step.event?.description ?? DEFAULT_DESCRIPTIONS[step.status][type];
            const ts = step.event?.timestamp ? formatTs(step.event.timestamp) : null;

            return (
              <div key={step.status} style={{ display: "flex", gap: 16, position: "relative", paddingBottom: isLast ? 0 : 32 }}>
                {/* Connector line */}
                {!isLast && (
                  <ProgressBar
                    from={step.m.color}
                    to={nextMeta!.color}
                    active={step.isActive}
                    done={step.isDone && !step.isActive}
                  />
                )}

                {/* Node */}
                <NodeIcon
                  status={step.status}
                  active={step.isActive}
                  done={step.isDone}
                  failed={false}
                />

                {/* Content */}
                <div style={{ flex: 1, paddingTop: 8, minWidth: 0 }}>
                  <div
                    onClick={() => setExpandedStep(expandedStep === step.status ? null : step.status)}
                    style={{ cursor: "pointer", userSelect: "none" }}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                      <span style={{
                        ...sans, fontSize: 13, fontWeight: 600,
                        color: step.isDone || step.isActive ? "var(--ak-text)" : "var(--ak-text-subtle)",
                        transition: "color 0.4s",
                      }}>
                        {step.event?.label ?? step.m.label}
                      </span>
                      {step.isDone && !step.isActive && (
                        <span style={{
                          ...mono, fontSize: 9, fontWeight: 700, letterSpacing: "0.12em",
                          padding: "2px 7px", borderRadius: 10,
                          background: step.m.bg, color: step.m.color,
                          border: `1px solid ${step.m.border}`,
                        }}>DONE</span>
                      )}
                      {step.isActive && (
                        <span style={{
                          ...mono, fontSize: 9, fontWeight: 700, letterSpacing: "0.12em",
                          padding: "2px 7px", borderRadius: 10,
                          background: step.m.bg, color: step.m.color,
                          border: `1px solid ${step.m.border}`,
                          animation: "txs-fade-in-out 1.8s ease-in-out infinite",
                        }}>LIVE</span>
                      )}
                    {/* Expand toggle — only when step has event data */}
                    {step.event && (
                      <button
                        aria-expanded={expandedStep === step.status}
                        aria-controls={`step-details-${step.status}`}
                        onClick={() => toggleStep(step.status)}
                        onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggleStep(step.status); } }}
                        style={{
                          ...mono, marginLeft: "auto", fontSize: 10, fontWeight: 600,
                          padding: "2px 8px", borderRadius: 8,
                          border: `1px solid var(--ak-border)`,
                          background: "var(--ak-surface-2)",
                          color: "var(--ak-text-muted)",
                          cursor: "pointer",
                          transition: "all 0.15s",
                          flexShrink: 0,
                        }}
                      >
                        {expandedStep === step.status ? "▲ Less" : "▼ More"}
                      </button>
                    )}
                  </div>

                  <p style={{
                    ...sans, fontSize: 12, color: step.isDone || step.isActive ? "var(--ak-text-muted)" : "var(--ak-text-subtle)",
                    lineHeight: 1.55, margin: 0,
                    transition: "color 0.4s",
                  }}>
                    {desc}
                  </p>

                  {/* Extra metadata row */}
                  <div style={{ display: "flex", flexWrap: "wrap", gap: "4px 12px", marginTop: 6 }}>
                    {ts && (
                      <span style={{ ...mono, fontSize: 10, color: "var(--ak-text-muted)" }}>{ts}</span>
                    )}
                    {step.event?.txHash && (
                      <a
                        href={`https://stellar.expert/explorer/testnet/tx/${step.event.txHash}`}
                        target="_blank" rel="noreferrer"
                        style={{ ...mono, fontSize: 10, color: "var(--ak-status-initiated-color)", textDecoration: "none", display: "flex", alignItems: "center", gap: 3 }}
                      >
                        ↗ {truncateHash(step.event.txHash)}
                      </a>
                    )}
                    {step.event?.detail && (
                      <span style={{ ...mono, fontSize: 10, color: "var(--ak-text-muted)" }}>{step.event.detail}</span>
                    )}
                  </div>

                  {/* Expanded details panel */}
                  {expandedStep === step.status && (
                    <div
                      id={`step-details-${step.status}`}
                      role="region"
                      aria-label={`${step.event?.label ?? step.m.label} details`}
                      style={{
                        marginTop: 10,
                        padding: "10px 12px",
                        borderRadius: 10,
                        border: "1px solid var(--ak-border)",
                        background: "var(--ak-surface-2)",
                        display: "flex", flexDirection: "column", gap: 8,
                      }}
                    >
                      {/* Timestamp */}
                      <div>
                        <span style={{ ...mono, fontSize: 9, fontWeight: 700, letterSpacing: "0.1em", textTransform: "uppercase", color: "var(--ak-text-muted)" }}>Timestamp</span>
                        <p style={{ ...mono, fontSize: 11, color: "var(--ak-text)", margin: "3px 0 0" }}>
                          {step.event?.timestamp ? formatTs(step.event.timestamp) : "—"}
                        </p>
                      </div>

                      {/* Error message */}
                      {(step.event?.errorMessage != null) && (
                        <div>
                          <span style={{ ...mono, fontSize: 9, fontWeight: 700, letterSpacing: "0.1em", textTransform: "uppercase", color: "var(--ak-status-failed-color)" }}>Error</span>
                          <p style={{ ...mono, fontSize: 11, color: "var(--ak-status-failed-color)", margin: "3px 0 0", wordBreak: "break-word" }}>
                            {step.event.errorMessage || "No error message provided."}
                          </p>
                        </div>
                      )}

                      {/* Raw API response */}
                      <div>
                        <span style={{ ...mono, fontSize: 9, fontWeight: 700, letterSpacing: "0.1em", textTransform: "uppercase", color: "var(--ak-text-muted)" }}>Raw API Response</span>
                        {step.event?.rawApiResponse != null ? (
                          <pre style={{
                            ...mono, fontSize: 10, color: "var(--ak-text)", margin: "3px 0 0",
                            background: "var(--ak-surface-3)", padding: "8px 10px", borderRadius: 7,
                            border: "1px solid var(--ak-border)",
                            overflowX: "auto", whiteSpace: "pre-wrap", wordBreak: "break-word",
                            maxHeight: 200,
                          }}>
                            {typeof step.event.rawApiResponse === "string"
                              ? step.event.rawApiResponse
                              : JSON.stringify(step.event.rawApiResponse, null, 2)}
                          </pre>
                        ) : (
                          <p style={{ ...mono, fontSize: 11, color: "var(--ak-text-subtle)", margin: "3px 0 0" }}>No response data available.</p>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            );
          })}

          {/* Failed node — appended after the last completed step */}
          {failed && (
            <div style={{ display: "flex", gap: 16, position: "relative" }}>
              <div style={{ position: "relative", width: 40, height: 40, flexShrink: 0 }}>
                <div style={{
                  width: 40, height: 40, borderRadius: "50%",
                  display: "flex", alignItems: "center", justifyContent: "center",
                  border: `2px solid var(--ak-status-failed-color)`,
                  background: "var(--ak-status-failed-bg)",
                  boxShadow: "0 0 0 4px var(--ak-status-failed-bg), 0 2px 12px rgba(220,38,38,0.2)",
                  fontSize: 16, color: "var(--ak-status-failed-color)", fontWeight: 700, zIndex: 1, position: "relative",
                  animation: "txs-shake 0.5s ease both",
                }}>✕</div>
              </div>
              <div style={{ flex: 1, paddingTop: 8 }}>
                <div
                  onClick={() => setExpandedStep(expandedStep === "failed" ? null : "failed")}
                  style={{ cursor: "pointer", userSelect: "none" }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                    <span style={{ ...sans, fontSize: 13, fontWeight: 600, color: "var(--ak-status-failed-color)" }}>
                      {events.find(e => e.status === "failed")?.label ?? "Transaction Failed"}
                    </span>
                    <span style={{ ...mono, fontSize: 10, color: "var(--ak-text-subtle)", marginLeft: "auto", display: "inline-block", transition: "transform 0.2s", transform: expandedStep === "failed" ? "rotate(180deg)" : "rotate(0deg)" }}>▾</span>
                  </div>
                  <p style={{ ...sans, fontSize: 12, color: "var(--ak-status-failed-color)", lineHeight: 1.55, margin: 0 }}>
                    {events.find(e => e.status === "failed")?.description ?? DEFAULT_DESCRIPTIONS.failed[type]}
                  </p>
                  {events.find(e => e.status === "failed")?.timestamp && (
                    <span style={{ ...mono, fontSize: 10, color: "var(--ak-status-failed-color)", marginTop: 4, display: "block" }}>
                      {formatTs(events.find(e => e.status === "failed")!.timestamp)}
                    </span>
                  )}
                </div>
                {expandedStep === "failed" && (() => {
                  const ev = events.find(e => e.status === "failed");
                  return (
                    <div style={{ marginTop: 10, padding: "12px 14px", borderRadius: 10, border: "1px solid var(--ak-status-failed-border)", background: "var(--ak-status-failed-bg)", display: "flex", flexDirection: "column", gap: 8 }}>
                      {ev?.timestamp && (
                        <div>
                          <span style={{ ...mono, fontSize: 9, letterSpacing: "0.1em", color: "var(--ak-status-failed-color)", textTransform: "uppercase", display: "block", marginBottom: 2 }}>Timestamp</span>
                          <span style={{ ...mono, fontSize: 11, color: "var(--ak-status-failed-color)" }}>{ev.timestamp}</span>
                        </div>
                      )}
                      {ev?.error && (
                        <div>
                          <span style={{ ...mono, fontSize: 9, letterSpacing: "0.1em", color: "var(--ak-status-failed-color)", textTransform: "uppercase", display: "block", marginBottom: 2 }}>Error</span>
                          <span style={{ ...mono, fontSize: 11, color: "var(--ak-status-failed-color)" }}>{ev.error}</span>
                        </div>
                      )}
                      {ev?.rawApiResponse !== undefined && (
                        <div>
                          <span style={{ ...mono, fontSize: 9, letterSpacing: "0.1em", color: "var(--ak-status-failed-color)", textTransform: "uppercase", display: "block", marginBottom: 2 }}>Raw API Response</span>
                          <pre style={{ ...mono, fontSize: 10, color: "var(--ak-status-failed-color)", background: "var(--ak-surface-3)", padding: "10px 12px", borderRadius: 8, border: "1px solid var(--ak-status-failed-border)", overflowX: "auto", margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-all", maxHeight: 200, overflowY: "auto" }}>
                            {JSON.stringify(ev.rawApiResponse, null, 2)}
                          </pre>
                        </div>
                      )}
                    </div>
                  );
                })()}
              </div>
            </div>
          )}

          {/* Refunded node — appended after steps; distinct from failed (funds are returning) */}
          {refunded && (
            <div style={{ display: "flex", gap: 16, position: "relative" }}>
              <div style={{ position: "relative", width: 40, height: 40, flexShrink: 0 }}>
                <div style={{
                  width: 40, height: 40, borderRadius: "50%",
                  display: "flex", alignItems: "center", justifyContent: "center",
                  border: `2px solid var(--ak-status-refunded-color)`,
                  background: "var(--ak-status-refunded-bg)",
                  boxShadow: "0 0 0 4px var(--ak-status-refunded-bg), 0 2px 12px rgba(124,58,237,0.2)",
                  fontSize: 16, color: "var(--ak-status-refunded-color)", fontWeight: 700, zIndex: 1, position: "relative",
                }}>↩</div>
              </div>
              <div style={{ flex: 1, paddingTop: 8 }}>
                <div style={{ ...sans, fontSize: 13, fontWeight: 600, color: "var(--ak-status-refunded-color-strong)", marginBottom: 4 }}>
                  {events.find(e => e.status === "refunded")?.label ?? "Transaction Refunded"}
                </div>
                <p style={{ ...sans, fontSize: 12, color: "var(--ak-status-refunded-color)", lineHeight: 1.55, margin: 0 }}>
                  {events.find(e => e.status === "refunded")?.description ?? DEFAULT_DESCRIPTIONS.refunded[type]}
                </p>
                {events.find(e => e.status === "refunded")?.timestamp && (
                  <span style={{ ...mono, fontSize: 10, color: "var(--ak-status-refunded-color-subtle)", marginTop: 4, display: "block" }}>
                    {formatTs(events.find(e => e.status === "refunded")!.timestamp)}
                  </span>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* ── Footer actions ── */}
      {(onRetry || onClose) && (
        <div style={{
          padding: "14px 24px 20px",
          borderTop: "1px solid var(--ak-border)",
          display: "flex", gap: 10,
        }}>
          {onRetry && failed && (
            <button onClick={onRetry} style={{
              ...sans, flex: 1, padding: "10px 0",
              borderRadius: 10, border: "1.5px solid var(--ak-status-failed-color)",
              background: "var(--ak-status-failed-bg)", color: "var(--ak-status-failed-color)",
              fontSize: 13, fontWeight: 600, cursor: "pointer",
              transition: "all 0.2s",
            }}>
              ↺ Retry
            </button>
          )}
          {onClose && (
            <button onClick={onClose} style={{
              ...sans, flex: 1, padding: "10px 0",
              borderRadius: 10, border: "1.5px solid var(--ak-border)",
              background: "var(--ak-surface-2)", color: "var(--ak-text-muted)",
              fontSize: 13, fontWeight: 600, cursor: "pointer",
              transition: "all 0.2s",
            }}>
              Close
            </button>
          )}
          {completed && (
            <button onClick={onClose} style={{
              ...sans, flex: 1, padding: "10px 0",
              borderRadius: 10, border: "none",
              background: "linear-gradient(135deg,#059669,#10b981)",
              color: "#fff",
              fontSize: 13, fontWeight: 600, cursor: "pointer",
              boxShadow: "0 2px 12px rgba(5,150,105,0.3)",
              transition: "all 0.2s",
            }}>
              Done
            </button>
          )}
        </div>
      )}

      {/* Keyframes */}
      <style>{`
        @keyframes txs-ping        { 0%{transform:scale(1);opacity:.8} 100%{transform:scale(2.2);opacity:0} }
        @keyframes txs-ping-slow   { 0%,100%{opacity:1} 50%{opacity:0.4} }
        @keyframes txs-spin        { to{transform:rotate(360deg)} }
        @keyframes txs-dot-pulse   { 0%,80%,100%{transform:scale(0.6);opacity:.4} 40%{transform:scale(1.1);opacity:1} }
        @keyframes txs-drip        { 0%,100%{transform:translate(-50%,-50%) scale(1);opacity:.6} 50%{transform:translate(-50%,200%) scale(0.4);opacity:0} }
        @keyframes txs-fade-in-out { 0%,100%{opacity:1} 50%{opacity:0.45} }
        @keyframes txs-shake       { 0%,100%{transform:translateX(0)} 20%,60%{transform:translateX(-4px)} 40%,80%{transform:translateX(4px)} }
      `}</style>
    </div>
  );
}


