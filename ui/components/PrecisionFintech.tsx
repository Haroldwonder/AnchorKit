import { useState } from "react";
import './themes.css';
import './precision-fintech-tokens.css';

const metrics = [
  { label: "Portfolio Value", value: "$2,847,391", change: "+4.21%", up: true },
  { label: "Daily P&L", value: "+$12,043", change: "+0.43%", up: true },
  { label: "Sharpe Ratio", value: "2.84", change: "-0.07", up: false },
  { label: "Alpha", value: "18.3%", change: "+1.2%", up: true },
];

const positions = [
  {
    ticker: "NVDA",
    name: "NVIDIA Corp",
    qty: 240,
    price: 874.15,
    change: 3.21,
    value: 209796,
  },
  {
    ticker: "AAPL",
    name: "Apple Inc",
    qty: 500,
    price: 189.32,
    change: -0.44,
    value: 94660,
  },
  {
    ticker: "MSFT",
    name: "Microsoft",
    qty: 310,
    price: 415.7,
    change: 1.87,
    value: 128867,
  },
  {
    ticker: "BTC",
    name: "Bitcoin",
    qty: 2.4,
    price: 68400,
    change: 5.12,
    value: 164160,
  },
  {
    ticker: "GLD",
    name: "Gold ETF",
    qty: 180,
    price: 222.1,
    change: -0.11,
    value: 39978,
  },
];

const chartData = [
  32, 48, 41, 55, 47, 62, 58, 71, 65, 80, 74, 88, 82, 95, 91, 100,
];

export default function PrecisionFintech() {
  const [activeTab, setActiveTab] = useState("overview");
  const [hoveredRow, setHoveredRow] = useState<number | null>(null);

  const maxChart = Math.max(...chartData);
  const minChart = Math.min(...chartData);

  return (
    <div
      className="min-h-screen w-full text-white"
      style={{
        background: "var(--ak-bg)",
        fontFamily: "var(--pf-font-mono)",
        color: "var(--ak-text)",
      }}
    >
      <style>{`
        .mint { color: var(--pf-mint); }
        .mint-bg { background: var(--pf-mint); }

        .glass {
          background: var(--ak-surface);
          border: 1px solid var(--ak-border);
        }

        .glass-mint {
          background: var(--pf-mint-dim);
          border: 1px solid var(--pf-mint-border);
        }

        .btn-primary {
          background: var(--pf-mint);
          color: var(--pf-bg);
          font-weight: 500;
          letter-spacing: 0.08em;
          text-transform: uppercase;
          font-size: var(--pf-font-size-base);
          padding: 10px var(--pf-spacing-2xl);
          border: none;
          cursor: pointer;
          transition: var(--pf-transition);
          font-family: var(--pf-font-mono);
        }
        .btn-primary:hover {
          background: var(--pf-mint-hover);
          transform: translateY(-1px);
          box-shadow: 0 0 24px var(--pf-mint-glow);
        }

        .btn-ghost {
          background: transparent;
          color: var(--pf-mint);
          border: 1px solid var(--pf-mint-border);
          font-weight: 400;
          letter-spacing: 0.08em;
          text-transform: uppercase;
          font-size: var(--pf-font-size-base);
          padding: 10px var(--pf-spacing-2xl);
          cursor: pointer;
          transition: var(--pf-transition);
          font-family: var(--pf-font-mono);
        }
        .btn-ghost:hover {
          background: var(--pf-mint-dim);
          border-color: var(--pf-mint);
        }

        .tab {
          background: transparent;
          border: none;
          color: var(--ak-text-muted);
          font-family: var(--pf-font-mono);
          font-size: var(--pf-font-size-base);
          letter-spacing: 0.1em;
          text-transform: uppercase;
          padding: var(--pf-spacing-sm) var(--pf-spacing-lg);
          cursor: pointer;
          transition: var(--pf-transition);
          border-bottom: 2px solid transparent;
        }
        .tab.active {
          color: var(--pf-mint);
          border-bottom-color: var(--pf-mint);
        }
        .tab:hover:not(.active) {
          color: var(--ak-text);
        }

        .metric-card {
          background: var(--ak-surface);
          border: 1px solid var(--ak-border);
          padding: var(--pf-spacing-2xl);
          transition: border-color 0.2s;
        }
        .metric-card:hover {
          border-color: var(--pf-mint-border);
        }

        .row-hover:hover {
          background: var(--pf-mint-dim) !important;
        }

        .up { color: var(--pf-mint); }
        .dn { color: var(--pf-danger); }

        .pulse {
          width: 7px;
          height: 7px;
          background: var(--pf-mint);
          border-radius: 50%;
          display: inline-block;
          animation: pulse 2s infinite;
        }
        @keyframes pulse {
          0%, 100% { box-shadow: 0 0 0 0 var(--pf-mint-pulse-start); }
          50% { box-shadow: 0 0 0 6px var(--pf-mint-pulse-end); }
        }

        .scanline {
          position: absolute;
          inset: 0;
          background: repeating-linear-gradient(
            0deg,
            transparent,
            transparent 3px,
            rgba(0,255,178,0.012) 3px,
            rgba(0,255,178,0.012) 4px
          );
          pointer-events: none;
        }
      `}</style>

      {/* Scanline overlay */}
      <div className="fixed inset-0 pointer-events-none" style={{ zIndex: 0 }}>
        <div className="scanline" />
      </div>

      <div className="relative" style={{ zIndex: 1 }}>
        {/* Header */}
        <header
          style={{ borderBottom: "1px solid var(--ak-border)" }}
          className="flex items-center justify-between px-8 py-5"
        >
          <div className="flex items-center gap-8">
            <div>
              <div
                style={{
                  fontFamily: "var(--pf-font-display)",
                  fontWeight: 800,
                  fontSize: 18,
                  letterSpacing: "-0.01em",
                }}
              >
                PREC<span className="mint">.</span>IO
              </div>
              <div
                style={{ fontSize: 9, color: "var(--ak-text-muted)", letterSpacing: "0.2em" }}
              >
                PRECISION FINTECH
              </div>
            </div>
            <div style={{ width: 1, height: 32, background: "var(--ak-border)" }} />
            <div className="flex items-center gap-2">
              <span className="pulse" />
              <span
                style={{ fontSize: 10, color: "var(--ak-text-muted)", letterSpacing: "0.15em" }}
              >
                MARKETS OPEN
              </span>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <div
              style={{ fontSize: 11, color: "var(--ak-text-muted)", letterSpacing: "0.05em" }}
            >
              USD / ACCT_9841
            </div>
            <button className="btn-ghost">Withdraw</button>
            <button className="btn-primary">+ Deploy Capital</button>
          </div>
        </header>

        <div className="px-8 py-8">
          {/* Headline */}
          <div className="mb-10">
            <div
              style={{
                fontSize: 10,
                color: "var(--ak-text-muted)",
                letterSpacing: "0.25em",
                marginBottom: 8,
              }}
            >
              PORTFOLIO OVERVIEW — FEB 24, 2026
            </div>
            <div
              style={{
                fontFamily: "var(--pf-font-display)",
                fontWeight: 800,
                fontSize: 48,
                lineHeight: 1,
                letterSpacing: "-0.03em",
              }}
            >
              $2,847,<span className="mint">391</span>
            </div>
            <div className="flex items-center gap-3 mt-3">
              <span style={{ fontSize: 13, color: "var(--ak-text-muted)" }}>Total AUM</span>
              <span
                style={{
                  width: 1,
                  height: 12,
                  background: "var(--ak-border)",
                  display: "inline-block",
                }}
              />
              <span className="up" style={{ fontSize: 13 }}>
                ▲ +$12,043 today
              </span>
              <span
                className="up"
                style={{ fontSize: 11, color: "rgba(0,255,178,0.5)" }}
              >
                +0.43%
              </span>
            </div>
          </div>

          {/* Metrics */}
          <div className="grid grid-cols-4 gap-3 mb-8">
            {metrics.map((m) => (
              <div key={m.label} className="metric-card">
                <div
                  style={{
                    fontSize: 9,
                    color: "var(--ak-text-muted)",
                    letterSpacing: "0.2em",
                    marginBottom: 16,
                  }}
                >
                  {m.label.toUpperCase()}
                </div>
                <div
                  style={{
                    fontFamily: "var(--pf-font-display)",
                    fontWeight: 700,
                    fontSize: 26,
                    letterSpacing: "-0.02em",
                    marginBottom: 8,
                  }}
                >
                  {m.value}
                </div>
                <div className={m.up ? "up" : "dn"} style={{ fontSize: 11 }}>
                  {m.up ? "▲" : "▼"} {m.change}
                </div>
              </div>
            ))}
          </div>

          <div
            className="grid gap-6"
            style={{ gridTemplateColumns: "1fr 340px" }}
          >
            {/* Left column */}
            <div className="flex flex-col gap-6">
              {/* Chart */}
              <div className="glass" style={{ padding: "28px" }}>
                <div className="flex items-center justify-between mb-8">
                  <div>
                    <div
                      style={{
                        fontSize: 9,
                        letterSpacing: "0.2em",
                        color: "var(--ak-text-muted)",
                        marginBottom: 4,
                      }}
                    >
                      PERFORMANCE
                    </div>
                    <div
                      style={{
                        fontFamily: "var(--pf-font-display)",
                        fontWeight: 700,
                        fontSize: 18,
                      }}
                    >
                      Portfolio Return
                    </div>
                  </div>
                  <div className="flex gap-1">
                    {["1D", "1W", "1M", "3M", "YTD"].map((p, i) => (
                      <button
                        key={p}
                        style={{
                          fontSize: 10,
                          letterSpacing: "0.1em",
                          padding: "4px 10px",
                          border: "1px solid",
                          borderColor: i === 2 ? "var(--pf-mint)" : "var(--ak-border)",
                          color: i === 2 ? "var(--pf-mint)" : "var(--ak-text-muted)",
                          background:
                            i === 2 ? "var(--mint-dim)" : "transparent",
                          cursor: "pointer",
                          fontFamily: "var(--pf-font-mono)",
                        }}
                      >
                        {p}
                      </button>
                    ))}
                  </div>
                </div>

                {/* SVG Chart */}
                <div style={{ position: "relative", height: 140 }}>
                  <svg
                    width="100%"
                    height="140"
                    viewBox={`0 0 ${chartData.length * 40} 120`}
                    preserveAspectRatio="none"
                  >
                    <defs>
                      <linearGradient
                        id="chartGrad"
                        x1="0"
                        y1="0"
                        x2="0"
                        y2="1"
                      >
                        <stop
                          offset="0%"
                          stopColor="#00FFB2"
                          stopOpacity="0.18"
                        />
                        <stop
                          offset="100%"
                          stopColor="#00FFB2"
                          stopOpacity="0"
                        />
                      </linearGradient>
                    </defs>
                    {/* Area fill */}
                    <path
                      d={[
                        ...chartData.map((v, i) => {
                          const x = i * 40 + 20;
                          const y =
                            110 -
                            ((v - minChart) / (maxChart - minChart)) * 100;
                          return `${i === 0 ? "M" : "L"} ${x} ${y}`;
                        }),
                        `L ${(chartData.length - 1) * 40 + 20} 120`,
                        `L 20 120 Z`,
                      ].join(" ")}
                      fill="url(#chartGrad)"
                    />
                    {/* Line */}
                    <path
                      d={chartData
                        .map((v, i) => {
                          const x = i * 40 + 20;
                          const y =
                            110 -
                            ((v - minChart) / (maxChart - minChart)) * 100;
                          return `${i === 0 ? "M" : "L"} ${x} ${y}`;
                        })
                        .join(" ")}
                      stroke="#00FFB2"
                      strokeWidth="2"
                      fill="none"
                    />
                    {/* Last dot */}
                    <circle
                      cx={(chartData.length - 1) * 40 + 20}
                      cy={
                        110 -
                        ((chartData[chartData.length - 1] - minChart) /
                          (maxChart - minChart)) *
                          100
                      }
                      r="4"
                      fill="#00FFB2"
                    />
                  </svg>
                  {/* Y labels */}
                  <div
                    style={{
                      position: "absolute",
                      top: 0,
                      right: 0,
                      display: "flex",
                      flexDirection: "column",
                      justifyContent: "space-between",
                      height: "100%",
                      fontSize: 9,
                      color: "var(--ak-text-muted)",
                      letterSpacing: "0.1em",
                      pointerEvents: "none",
                    }}
                  >
                    <span>+18.3%</span>
                    <span>+9.1%</span>
                    <span>0%</span>
                  </div>
                </div>

                <div
                  className="flex justify-between mt-4"
                  style={{
                    fontSize: 9,
                    color: "var(--ak-text-muted)",
                    letterSpacing: "0.05em",
                  }}
                >
                  {[
                    "JAN",
                    "FEB",
                    "MAR",
                    "APR",
                    "MAY",
                    "JUN",
                    "JUL",
                    "AUG",
                    "SEP",
                    "OCT",
                    "NOV",
                    "DEC",
                    "JAN",
                    "FEB",
                    "MAR",
                    "APR",
                  ].map((m, i) => (
                    <span key={i}>{m}</span>
                  ))}
                </div>
              </div>

              {/* Positions table */}
              <div className="glass">
                <div
                  className="flex items-center justify-between px-6 py-5"
                  style={{ borderBottom: "1px solid var(--ak-border)" }}
                >
                  <div
                    style={{
                      fontFamily: "var(--pf-font-display)",
                      fontWeight: 700,
                      fontSize: 15,
                    }}
                  >
                    Positions
                  </div>
                  <div className="flex gap-1">
                    {["overview", "analytics", "history"].map((t) => (
                      <button
                        key={t}
                        className={`tab ${activeTab === t ? "active" : ""}`}
                        onClick={() => setActiveTab(t)}
                      >
                        {t}
                      </button>
                    ))}
                  </div>
                </div>

                <table style={{ width: "100%", borderCollapse: "collapse" }}>
                  <thead>
                    <tr style={{ borderBottom: "1px solid var(--ak-border)" }}>
                      {[
                        "Asset",
                        "Quantity",
                        "Price",
                        "Change",
                        "Value",
                        "Action",
                      ].map((h) => (
                        <th
                          key={h}
                          style={{
                            padding: "10px 24px",
                            textAlign: "left",
                            fontSize: 9,
                            color: "var(--ak-text-muted)",
                            letterSpacing: "0.18em",
                            fontWeight: 400,
                          }}
                        >
                          {h.toUpperCase()}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {positions.map((p, i) => (
                      <tr
                        key={p.ticker}
                        className="row-hover"
                        onMouseEnter={() => setHoveredRow(i)}
                        onMouseLeave={() => setHoveredRow(null)}
                        style={{
                          borderBottom: "1px solid #0d0d0d",
                          cursor: "pointer",
                          transition: "background 0.1s",
                        }}
                      >
                        <td style={{ padding: "16px 24px" }}>
                          <div
                            style={{
                              fontFamily: "var(--pf-font-display)",
                              fontWeight: 700,
                              fontSize: 14,
                            }}
                          >
                            {p.ticker}
                          </div>
                          <div
                            style={{
                              fontSize: 10,
                              color: "var(--ak-text-muted)",
                              marginTop: 2,
                            }}
                          >
                            {p.name}
                          </div>
                        </td>
                        <td
                          style={{
                            padding: "16px 24px",
                            fontSize: 13,
                            color: "var(--ak-text-muted)",
                          }}
                        >
                          {p.qty.toLocaleString()}
                        </td>
                        <td style={{ padding: "16px 24px", fontSize: 13 }}>
                          ${p.price.toLocaleString()}
                        </td>
                        <td
                          style={{ padding: "16px 24px", fontSize: 13 }}
                          className={p.change > 0 ? "up" : "dn"}
                        >
                          {p.change > 0 ? "+" : ""}
                          {p.change}%
                        </td>
                        <td
                          style={{
                            padding: "16px 24px",
                            fontFamily: "var(--pf-font-display)",
                            fontWeight: 700,
                            fontSize: 14,
                          }}
                        >
                          ${p.value.toLocaleString()}
                        </td>
                        <td style={{ padding: "16px 24px" }}>
                          <div className="flex gap-2">
                            <button
                              className="btn-primary"
                              style={{ padding: "6px 14px" }}
                            >
                              Buy
                            </button>
                            <button
                              className="btn-ghost"
                              style={{ padding: "6px 14px" }}
                            >
                              Sell
                            </button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>

            {/* Right column */}
            <div className="flex flex-col gap-4">
              {/* Quick trade */}
              <div className="glass-mint" style={{ padding: 24 }}>
                <div
                  style={{
                    fontSize: 9,
                    letterSpacing: "0.25em",
                    color: "var(--pf-mint)",
                    marginBottom: 20,
                  }}
                >
                  QUICK EXECUTE
                </div>
                <div className="flex gap-2 mb-5">
                  {["BUY", "SELL", "SWAP"].map((a, i) => (
                    <button
                      key={a}
                      style={{
                        flex: 1,
                        padding: "8px 0",
                        fontSize: 10,
                        letterSpacing: "0.1em",
                        background: i === 0 ? "var(--pf-mint)" : "transparent",
                        color: i === 0 ? "#000" : "rgba(0,255,178,0.5)",
                        border: "1px solid",
                        borderColor:
                          i === 0 ? "var(--pf-mint)" : "var(--pf-mint-border)",
                        cursor: "pointer",
                        fontFamily: "var(--pf-font-mono)",
                      }}
                    >
                      {a}
                    </button>
                  ))}
                </div>
                <div style={{ marginBottom: 12 }}>
                  <div
                    style={{
                      fontSize: 9,
                      color: "var(--ak-text-muted)",
                      letterSpacing: "0.15em",
                      marginBottom: 6,
                    }}
                  >
                    ASSET
                  </div>
                  <div
                    style={{
                      background: "#000",
                      border: "1px solid var(--ak-border)",
                      padding: "10px 14px",
                      fontSize: 13,
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                    }}
                  >
                    <span>NVDA</span>
                    <span
                      style={{
                        fontSize: 9,
                        color: "var(--ak-text-muted)",
                        letterSpacing: "0.1em",
                      }}
                    >
                      ▼
                    </span>
                  </div>
                </div>
                <div style={{ marginBottom: 16 }}>
                  <div
                    style={{
                      fontSize: 9,
                      color: "var(--ak-text-muted)",
                      letterSpacing: "0.15em",
                      marginBottom: 6,
                    }}
                  >
                    AMOUNT (USD)
                  </div>
                  <div
                    style={{
                      background: "#000",
                      border: "1px solid var(--mint-border)",
                      padding: "10px 14px",
                      fontSize: 13,
                      color: "var(--pf-mint)",
                      display: "flex",
                      justifyContent: "space-between",
                    }}
                  >
                    <span>10,000.00</span>
                    <span style={{ fontSize: 9, color: "var(--ak-text-muted)" }}>
                      ≈ 11.43 shares
                    </span>
                  </div>
                </div>
                <div
                  style={{
                    fontSize: 9,
                    color: "var(--ak-text-muted)",
                    letterSpacing: "0.1em",
                    marginBottom: 16,
                    display: "flex",
                    justifyContent: "space-between",
                  }}
                >
                  <span>Est. fee</span>
                  <span>$2.40</span>
                </div>
                <button
                  className="btn-primary"
                  style={{ width: "100%", padding: "13px 0", fontSize: 12 }}
                >
                  Execute Order
                </button>
              </div>

              {/* Allocation */}
              <div className="glass" style={{ padding: 24 }}>
                <div
                  style={{
                    fontSize: 9,
                    letterSpacing: "0.25em",
                    color: "var(--ak-text-muted)",
                    marginBottom: 20,
                  }}
                >
                  ALLOCATION
                </div>
                {[
                  { label: "Equities", pct: 66, val: "$635K" },
                  { label: "Crypto", pct: 25, val: "$242K" },
                  { label: "Commodities", pct: 9, val: "$86K" },
                ].map((a) => (
                  <div key={a.label} style={{ marginBottom: 14 }}>
                    <div
                      className="flex justify-between mb-2"
                      style={{ fontSize: 11, color: "var(--ak-text-muted)" }}
                    >
                      <span>{a.label}</span>
                      <span style={{ color: "#666" }}>
                        {a.val} <span className="mint">{a.pct}%</span>
                      </span>
                    </div>
                    <div
                      style={{
                        height: 3,
                        background: "#111",
                        position: "relative",
                      }}
                    >
                      <div
                        style={{
                          position: "absolute",
                          top: 0,
                          left: 0,
                          height: "100%",
                          width: `${a.pct}%`,
                          background: "var(--pf-mint)",
                          transition: "width 0.5s",
                        }}
                      />
                    </div>
                  </div>
                ))}
              </div>

              {/* Alerts */}
              <div className="glass" style={{ padding: 24 }}>
                <div
                  style={{
                    fontSize: 9,
                    letterSpacing: "0.25em",
                    color: "var(--ak-text-muted)",
                    marginBottom: 16,
                  }}
                >
                  SYSTEM ALERTS
                </div>
                {[
                  {
                    msg: "NVDA stop-loss triggered at $870",
                    time: "2m ago",
                    severity: "warn",
                  },
                  {
                    msg: "BTC target hit: +5.1%",
                    time: "18m ago",
                    severity: "ok",
                  },
                  {
                    msg: "Margin utilization at 72%",
                    time: "1h ago",
                    severity: "warn",
                  },
                ].map((alert, i) => (
                  <div
                    key={i}
                    style={{
                      padding: "10px 14px",
                      marginBottom: 8,
                      background:
                        alert.severity === "ok"
                          ? "var(--pf-mint-dim)"
                          : "rgba(255,77,109,0.05)",
                      borderLeft: `2px solid ${alert.severity === "ok" ? "var(--pf-mint)" : "var(--pf-danger)"}`,
                      fontSize: 11,
                    }}
                  >
                    <div
                      style={{
                        color:
                          alert.severity === "ok" ? "var(--pf-mint)" : "var(--pf-danger)",
                        marginBottom: 3,
                      }}
                    >
                      {alert.msg}
                    </div>
                    <div
                      style={{
                        fontSize: 9,
                        color: "var(--ak-text-muted)",
                        letterSpacing: "0.1em",
                      }}
                    >
                      {alert.time}
                    </div>
                  </div>
                ))}
                <button
                  className="btn-ghost"
                  style={{ width: "100%", marginTop: 4, padding: "8px 0" }}
                >
                  View All Alerts
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <footer
          style={{
            borderTop: "1px solid #0f0f0f",
            padding: "16px 32px",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <div
            style={{ fontSize: 9, color: "#2a2a2a", letterSpacing: "0.15em" }}
          >
            PREC.IO FINTECH PLATFORM — REGULATED ENTITY — ISO 27001 CERTIFIED
          </div>
          <div
            style={{ fontSize: 9, color: "#2a2a2a", letterSpacing: "0.15em" }}
          >
            ALL DATA ENCRYPTED END-TO-END
          </div>
        </footer>
      </div>
    </div>
  );
}
