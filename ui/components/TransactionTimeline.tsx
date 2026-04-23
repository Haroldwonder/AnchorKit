import { useState, useEffect, useRef } from "react";

// ─── Types ────────────────────────────────────────────────────────────────────

export type TxStatus = "initiated" | "pending" | "processing" | "completed" | "failed";
export type TxType   = "deposit" | "withdrawal";

export interface TxEvent {
  status: TxStatus;
  label?: string;
  description?: string;
  timestamp?: string;         // ISO string or readable label
  txHash?: string;            // on-chain hash, shown on completed
  detail?: string;            // any extra note
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

const STATUS_ORDER: TxStatus[] = ["initiated", "pending", "processing", "completed"];

const STATUS_META: Record<TxStatus, { label: string; color: string; bg: string; border: string; icon: string }> = {
  initiated:  { label: "Initiated",  color: "#6366f1", bg: "#eef2ff", border: "#c7d2fe", icon: "◎"  },
  pending:    { label: "Pending",    color: "#d97706", bg: "#fffbeb", border: "#fde68a", icon: "◌"  },
  processing: { label: "Processing", color: "#0284c7", bg: "#e0f2fe", border: "#bae6fd", icon: "◈"  },
  completed:  { label: "Completed",  color: "#059669", bg: "#ecfdf5", border: "#a7f3d0", icon: "✓"  },
  failed:     { label: "Failed",     color: "#dc2626", bg: "#fef2f2", border: "#fecaca", icon: "✕"  },
};

const DEFAULT_DESCRIPTIONS: Record<TxStatus, Record<TxType, string>> = {
  initiated:  { deposit: "Deposit request received by anchor.",        withdrawal: "Withdrawal request received by anchor."        },
  pending:    { deposit: "Awaiting your funds on the external rail.",  withdrawal: "Awaiting Stellar transaction confirmation."    },
  processing: { deposit: "Funds received — minting assets on-chain.", withdrawal: "Processing payment to your bank account."     },
  completed:  { deposit: "Assets delivered to your Stellar account.", withdrawal: "Funds sent to your destination account."      },
  failed:     { deposit: "Deposit could not be completed.",           withdrawal: "Withdrawal could not be completed."           },
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

function isFailed(status: TxStatus): boolean { return status === "failed"; }
function isCompleted(status: TxStatus): boolean { return status === "completed"; }
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
        border: `2px solid ${done || active ? m.color : "#e2e8f0"}`,
        background: done || active ? m.bg : "#f8fafc",
        boxShadow: active || done ? `0 0 0 4px ${m.bg}, 0 2px 12px ${m.color}28` : "none",
        transition: "all 0.5s cubic-bezier(0.4,0,0.2,1)",
        position: "relative", zIndex: 1,
        fontSize: done && !active ? 16 : 14,
        color: done || active ? m.color : "#cbd5e1",
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
      background: "#e2e8f0", borderRadius: 2, overflow: "hidden",
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
  const currentIndex = getOrderIndex(currentStatus);

  const [timeLeft, setTimeLeft] = useState<number | null>(null);

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
      background: "#fafaf9",
      borderRadius: 20,
      border: "1px solid #e7e5e0",
      boxShadow: "0 4px 24px rgba(0,0,0,0.07), 0 1px 4px rgba(0,0,0,0.04)",
      overflow: "hidden",
      maxWidth: 420,
      width: "100%",
    }}>

      {/* ── Header ── */}
      <div style={{
        padding: "22px 24px 20px",
        background: failed ? "#fef2f2" : completed ? "#ecfdf5" : "#f0f4ff",
        borderBottom: `1px solid ${failed ? "#fecaca" : completed ? "#a7f3d0" : "#dde5ff"}`,
        display: "flex", flexDirection: "column", gap: 14,
      }}>
        {/* Type pill + ID */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <span style={{
            ...mono, fontSize: 10, fontWeight: 600,
            letterSpacing: "0.12em", textTransform: "uppercase",
            padding: "3px 10px", borderRadius: 20,
            background: type === "deposit" ? "#dbeafe" : "#fce7f3",
            color: type === "deposit" ? "#1d4ed8" : "#9d174d",
            border: `1px solid ${type === "deposit" ? "#bfdbfe" : "#fbcfe8"}`,
          }}>
            {type === "deposit" ? "↓ Deposit" : "↑ Withdrawal"}
          </span>
          {id && (
            <span style={{ ...mono, fontSize: 10, color: "#94a3b8", letterSpacing: "0.06em" }}>
              #{id.slice(-8)}
            </span>
          )}
        </div>

        {/* Amount */}
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <span style={{ ...serif, fontSize: 36, fontWeight: 400, color: failed ? "#b91c1c" : completed ? "#065f46" : "#1e293b", lineHeight: 1 }}>
            {amount}
          </span>
          <span style={{ ...mono, fontSize: 14, color: "#64748b", fontWeight: 500 }}>{asset}</span>
        </div>

        {/* Status badge */}
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <div style={{
            width: 8, height: 8, borderRadius: "50%",
            background: statusMeta.color,
            boxShadow: `0 0 0 2px ${statusMeta.bg}`,
            animation: (!failed && !completed) ? "txs-ping-slow 2.5s ease-in-out infinite" : "none",
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
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                    <span style={{
                      ...sans, fontSize: 13, fontWeight: 600,
                      color: step.isDone || step.isActive ? "#0f172a" : "#94a3b8",
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
                  </div>

                  <p style={{
                    ...sans, fontSize: 12, color: step.isDone || step.isActive ? "#64748b" : "#cbd5e1",
                    lineHeight: 1.55, margin: 0,
                    transition: "color 0.4s",
                  }}>
                    {desc}
                  </p>

                  {/* Extra metadata row */}
                  <div style={{ display: "flex", flexWrap: "wrap", gap: "4px 12px", marginTop: 6 }}>
                    {ts && (
                      <span style={{ ...mono, fontSize: 10, color: "#94a3b8" }}>{ts}</span>
                    )}
                    {step.event?.txHash && (
                      <a
                        href={`https://stellar.expert/explorer/testnet/tx/${step.event.txHash}`}
                        target="_blank" rel="noreferrer"
                        style={{ ...mono, fontSize: 10, color: "#6366f1", textDecoration: "none", display: "flex", alignItems: "center", gap: 3 }}
                      >
                        ↗ {truncateHash(step.event.txHash)}
                      </a>
                    )}
                    {step.event?.detail && (
                      <span style={{ ...mono, fontSize: 10, color: "#94a3b8" }}>{step.event.detail}</span>
                    )}
                  </div>
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
                  border: `2px solid #dc2626`,
                  background: "#fef2f2",
                  boxShadow: "0 0 0 4px #fef2f2, 0 2px 12px rgba(220,38,38,0.2)",
                  fontSize: 16, color: "#dc2626", fontWeight: 700, zIndex: 1, position: "relative",
                  animation: "txs-shake 0.5s ease both",
                }}>✕</div>
              </div>
              <div style={{ flex: 1, paddingTop: 8 }}>
                <div style={{ ...sans, fontSize: 13, fontWeight: 600, color: "#b91c1c", marginBottom: 4 }}>
                  {events.find(e => e.status === "failed")?.label ?? "Transaction Failed"}
                </div>
                <p style={{ ...sans, fontSize: 12, color: "#ef4444", lineHeight: 1.55, margin: 0 }}>
                  {events.find(e => e.status === "failed")?.description ?? DEFAULT_DESCRIPTIONS.failed[type]}
                </p>
                {events.find(e => e.status === "failed")?.timestamp && (
                  <span style={{ ...mono, fontSize: 10, color: "#f87171", marginTop: 4, display: "block" }}>
                    {formatTs(events.find(e => e.status === "failed")!.timestamp)}
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
          borderTop: "1px solid #f1f5f9",
          display: "flex", gap: 10,
        }}>
          {onRetry && failed && (
            <button onClick={onRetry} style={{
              ...sans, flex: 1, padding: "10px 0",
              borderRadius: 10, border: "1.5px solid #dc2626",
              background: "#fef2f2", color: "#dc2626",
              fontSize: 13, fontWeight: 600, cursor: "pointer",
              transition: "all 0.2s",
            }}>
              ↺ Retry
            </button>
          )}
          {onClose && (
            <button onClick={onClose} style={{
              ...sans, flex: 1, padding: "10px 0",
              borderRadius: 10, border: "1.5px solid #e2e8f0",
              background: "#f8fafc", color: "#475569",
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
        @import url('https://fonts.googleapis.com/css2?family=DM+Serif+Display&family=DM+Sans:wght@400;500;600;700&family=IBM+Plex+Mono:wght@400;500;600&display=swap');
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

// ─── Demo Showcase ────────────────────────────────────────────────────────────

const DEMO_SCENARIOS: Array<{
  label: string;
  type: TxType;
  amount: string;
  asset: string;
  id: string;
  currentStatus: TxStatus;
  events: TxEvent[];
}> = [
  {
    label: "Deposit — Processing",
    type: "deposit", amount: "250.00", asset: "USDC",
    id: "dep_8f3c1a9e2b",
    currentStatus: "processing",
    estimatedCompletionAt: Date.now() + 185000,
    events: [
      { status: "initiated",  timestamp: new Date(Date.now() - 18 * 60000).toISOString(), detail: "via ACH transfer" },
      { status: "pending",    timestamp: new Date(Date.now() - 12 * 60000).toISOString(), detail: "Bank rail confirmed" },
      { status: "processing", timestamp: new Date(Date.now() - 2 * 60000).toISOString()  },
    ],
  },
  {
    label: "Deposit — Completed",
    type: "deposit", amount: "1,000.00", asset: "USDC",
    id: "dep_5a7d3f1b8c",
    currentStatus: "completed",
    events: [
      { status: "initiated",  timestamp: new Date(Date.now() - 60 * 60000).toISOString() },
      { status: "pending",    timestamp: new Date(Date.now() - 45 * 60000).toISOString() },
      { status: "processing", timestamp: new Date(Date.now() - 30 * 60000).toISOString() },
      { status: "completed",  timestamp: new Date(Date.now() - 5 * 60000).toISOString(), txHash: "4a7f8c3d2e1b9a6f5c0d3e8b1a4f7c2d5e8b1a4f7c2d9e6b3a0f5c8d1e4b7a", detail: "0.001 XLM fee" },
    ],
  },
  {
    label: "Withdrawal — Pending",
    type: "withdrawal", amount: "75.50", asset: "USDC",
    id: "wdl_2c8e5f9a1b",
    currentStatus: "pending",
    events: [
      { status: "initiated", timestamp: new Date(Date.now() - 5 * 60000).toISOString(), detail: "to SEPA account" },
      { status: "pending",   timestamp: new Date(Date.now() - 1 * 60000).toISOString(), description: "Stellar tx detected — awaiting finality." },
    ],
  },
  {
    label: "Withdrawal — Failed",
    type: "withdrawal", amount: "500.00", asset: "USDC",
    id: "wdl_9b3e7c1a4f",
    currentStatus: "failed",
    events: [
      { status: "initiated",  timestamp: new Date(Date.now() - 40 * 60000).toISOString() },
      { status: "pending",    timestamp: new Date(Date.now() - 35 * 60000).toISOString() },
      { status: "failed",     timestamp: new Date(Date.now() - 20 * 60000).toISOString(), description: "Destination bank rejected the transfer. Please verify your account details.", label: "Bank Rejected" },
    ],
  },
];

export default function TransactionTimelineDemo() {
  const [activeScenario, setActiveScenario] = useState(0);
  const [simulatedStatus, setSimulatedStatus] = useState<TxStatus | null>(null);
  const [simEvents, setSimEvents] = useState<TxEvent[]>([]);
  const [simRunning, setSimRunning] = useState(false);
  const simRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const scenario = DEMO_SCENARIOS[activeScenario];
  const isLive = simulatedStatus !== null;

  const displayStatus = isLive ? simulatedStatus! : scenario.currentStatus;
  const displayEvents  = isLive ? simEvents : scenario.events;

  const runSimulation = () => {
    if (simRunning) return;
    setSimRunning(true);
    setSimulatedStatus("initiated");
    setSimEvents([{ status: "initiated", timestamp: new Date().toISOString(), detail: "via ACH transfer" }]);

    const steps: [TxStatus, number, Partial<TxEvent>][] = [
      ["pending",    2200, { detail: "Bank rail received" }],
      ["processing", 4800, { description: "Funds received — minting on Stellar." }],
      ["completed",  7600, { txHash: "3d9f2a1c8e5b4f7a2d9c6e3b0f5a8d1c4e7b2a5f8c1d4e7b0a3f6c9d2e5b8a", detail: "0.001 XLM fee" }],
    ];

    steps.forEach(([status, delay, extra]) => {
      simRef.current = setTimeout(() => {
        setSimulatedStatus(status);
        setSimEvents(prev => [...prev, { status, timestamp: new Date().toISOString(), ...extra }]);
        if (status === "completed") setSimRunning(false);
      }, delay);
    });
  };

  const resetSim = () => {
    if (simRef.current) clearTimeout(simRef.current);
    setSimulatedStatus(null);
    setSimEvents([]);
    setSimRunning(false);
  };

  useEffect(() => { resetSim(); }, [activeScenario]);

  return (
    <div style={{
      ...sans,
      minHeight: "100vh",
      background: "linear-gradient(160deg, #f8f6f2 0%, #eef2f8 100%)",
      padding: "40px 24px",
    }}>
      <style>{`
        @import url('https://fonts.googleapis.com/css2?family=DM+Serif+Display&family=DM+Sans:wght@400;500;600;700&family=IBM+Plex+Mono:wght@400;500;600&display=swap');
        * { box-sizing: border-box; margin: 0; padding: 0; }
        button:hover { opacity: 0.88; }
      `}</style>

      <div style={{ maxWidth: 900, margin: "0 auto" }}>

        {/* Page header */}
        <div style={{ marginBottom: 40, textAlign: "center" }}>
          <div style={{ ...mono, fontSize: 11, letterSpacing: "0.2em", color: "#94a3b8", textTransform: "uppercase", marginBottom: 10 }}>
            Component / Timeline
          </div>
          <h1 style={{ ...serif, fontSize: 38, color: "#0f172a", fontWeight: 400, lineHeight: 1.15, marginBottom: 10 }}>
            Transaction Status Timeline
          </h1>
          <p style={{ ...sans, fontSize: 14, color: "#64748b", maxWidth: 440, margin: "0 auto", lineHeight: 1.6 }}>
            Reusable component for deposit & withdrawal flows. Handles all states with smooth animated transitions.
          </p>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 360px", gap: 32, alignItems: "start" }}>

          {/* Left — controls */}
          <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>

            {/* Scenario switcher */}
            <div>
              <div style={{ ...mono, fontSize: 10, letterSpacing: "0.16em", color: "#94a3b8", textTransform: "uppercase", marginBottom: 12 }}>
                Scenarios
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {DEMO_SCENARIOS.map((s, i) => {
                  const m = STATUS_META[s.currentStatus];
                  return (
                    <button key={i} onClick={() => setActiveScenario(i)} style={{
                      ...sans, display: "flex", alignItems: "center", gap: 12, padding: "12px 16px",
                      borderRadius: 12, border: `1.5px solid ${activeScenario === i ? m.border : "#e7e5e0"}`,
                      background: activeScenario === i ? m.bg : "#fff",
                      cursor: "pointer", textAlign: "left",
                      boxShadow: activeScenario === i ? `0 2px 12px ${m.color}18` : "0 1px 4px rgba(0,0,0,0.04)",
                      transition: "all 0.2s",
                    }}>
                      <div style={{ width: 8, height: 8, borderRadius: "50%", background: m.color, flexShrink: 0, boxShadow: `0 0 0 2px ${m.bg}` }} />
                      <span style={{ fontSize: 13, fontWeight: 500, color: activeScenario === i ? "#0f172a" : "#64748b" }}>{s.label}</span>
                      <span style={{ ...mono, fontSize: 10, color: m.color, marginLeft: "auto", fontWeight: 600 }}>{m.label}</span>
                    </button>
                  );
                })}
              </div>
            </div>

            {/* Live simulation */}
            <div style={{
              padding: 20, borderRadius: 16,
              border: "1.5px solid #e7e5e0",
              background: "#fff",
              boxShadow: "0 2px 12px rgba(0,0,0,0.04)",
            }}>
              <div style={{ ...mono, fontSize: 10, letterSpacing: "0.16em", color: "#94a3b8", textTransform: "uppercase", marginBottom: 12 }}>
                Live Simulation
              </div>
              <p style={{ ...sans, fontSize: 12, color: "#64748b", lineHeight: 1.6, marginBottom: 16 }}>
                Watch a deposit flow through all four states in real-time — progress bar fills, nodes activate, and the timeline builds itself.
              </p>
              <div style={{ display: "flex", gap: 8 }}>
                <button onClick={runSimulation} disabled={simRunning} style={{
                  ...sans, flex: 1, padding: "10px 0", borderRadius: 10,
                  border: "none", background: simRunning ? "#e2e8f0" : "linear-gradient(135deg,#6366f1,#4f46e5)",
                  color: simRunning ? "#94a3b8" : "#fff",
                  fontSize: 13, fontWeight: 600, cursor: simRunning ? "not-allowed" : "pointer",
                  boxShadow: simRunning ? "none" : "0 2px 12px rgba(99,102,241,0.35)",
                  transition: "all 0.2s",
                }}>
                  {simRunning ? "▶ Running…" : "▶ Start"}
                </button>
                <button onClick={resetSim} style={{
                  ...sans, padding: "10px 16px", borderRadius: 10,
                  border: "1.5px solid #e2e8f0", background: "#f8fafc",
                  color: "#64748b", fontSize: 13, fontWeight: 600, cursor: "pointer",
                  transition: "all 0.2s",
                }}>
                  ↺
                </button>
              </div>
              {isLive && (
                <div style={{ marginTop: 12, ...mono, fontSize: 10, color: "#6366f1",
                  padding: "6px 10px", borderRadius: 7, background: "#eef2ff", border: "1px solid #c7d2fe" }}>
                  ● Simulating live deposit… {STATUS_META[displayStatus].label}
                </div>
              )}
            </div>

            {/* Props reference */}
            <div style={{ padding: 20, borderRadius: 16, border: "1.5px solid #e7e5e0", background: "#fff", boxShadow: "0 2px 12px rgba(0,0,0,0.04)" }}>
              <div style={{ ...mono, fontSize: 10, letterSpacing: "0.16em", color: "#94a3b8", textTransform: "uppercase", marginBottom: 12 }}>
                Usage
              </div>
              <pre style={{ ...mono, fontSize: 10.5, color: "#334155", lineHeight: 1.75, background: "#f8fafc", padding: "14px", borderRadius: 10, border: "1px solid #e2e8f0", overflowX: "auto", whiteSpace: "pre-wrap" }}>{`<TransactionTimeline
  type="deposit"
  amount="250.00"
  asset="USDC"
  id="dep_8f3c1a"
  currentStatus="processing"
  events={[
    { status: "initiated",
      timestamp: "2024-01-15T10:30:00Z",
      detail: "via ACH" },
    { status: "pending",
      timestamp: "2024-01-15T10:42:00Z" },
    { status: "processing",
      timestamp: "2024-01-15T10:58:00Z" },
  ]}
  onRetry={() => {}}
  onClose={() => {}}
/>`}</pre>
            </div>
          </div>

          {/* Right — live preview */}
          <div style={{ position: "sticky", top: 24 }}>
            <div style={{ ...mono, fontSize: 10, letterSpacing: "0.16em", color: "#94a3b8", textTransform: "uppercase", marginBottom: 12 }}>
              Preview
            </div>
            <TransactionTimeline
              type={activeScenario === 0 || activeScenario === 1
                ? isLive ? "deposit" : scenario.type
                : scenario.type}
              amount={isLive ? "250.00" : scenario.amount}
              asset={scenario.asset}
              id={isLive ? "dep_live_demo" : scenario.id}
              currentStatus={displayStatus}
              events={displayEvents}
              onRetry={() => alert("Retry triggered")}
              onClose={() => alert("Close triggered")}
            />
          </div>
        </div>
      </div>
    </div>
  );
}