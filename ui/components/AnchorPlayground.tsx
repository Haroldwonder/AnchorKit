import { useState, useCallback, useRef, useEffect } from "react";

// ─── Types ────────────────────────────────────────────────────────────────────
type HttpMethod = "GET" | "POST" | "PUT";

interface Param {
  key: string;
  label: string;
  placeholder: string;
  required?: boolean;
}

interface Endpoint {
  id: string;
  method: HttpMethod;
  path: string;
  description: string;
  params: Param[];
}

interface SEPProtocol {
  id: string;
  name: string;
  tag: string;
  color: string;
  description: string;
  endpoints: Endpoint[];
}

interface HistoryEntry {
  id: string;
  time: string;
  sep: string;
  endpoint: string;
  method: HttpMethod;
  status: number | null;
  success: boolean;
}

// ─── SEP Data ─────────────────────────────────────────────────────────────────
const SEP_PROTOCOLS: SEPProtocol[] = [
  {
    id: "sep1",
    name: "SEP-1",
    tag: "Stellar.toml",
    color: "cyan",
    description: "Stellar Info File – exposes anchor metadata & capabilities",
    endpoints: [
      {
        id: "stellar-toml",
        method: "GET",
        path: "/.well-known/stellar.toml",
        description: "Fetch Stellar TOML config",
        params: [],
      },
    ],
  },
  {
    id: "sep6",
    name: "SEP-6",
    tag: "Deposit & Withdraw",
    color: "blue",
    description: "Programmatic deposit & withdrawal via query params",
    endpoints: [
      {
        id: "sep6-info",
        method: "GET",
        path: "/transfer/info",
        description: "Retrieve supported assets",
        params: [],
      },
      {
        id: "sep6-deposit",
        method: "GET",
        path: "/transfer/deposit",
        description: "Initiate a deposit",
        params: [
          {
            key: "asset_code",
            label: "Asset Code",
            placeholder: "USDC",
            required: true,
          },
          {
            key: "account",
            label: "Stellar Account",
            placeholder: "G...",
            required: true,
          },
          { key: "amount", label: "Amount", placeholder: "100" },
          { key: "memo", label: "Memo", placeholder: "optional memo" },
        ],
      },
      {
        id: "sep6-withdraw",
        method: "GET",
        path: "/transfer/withdraw",
        description: "Initiate a withdrawal",
        params: [
          {
            key: "asset_code",
            label: "Asset Code",
            placeholder: "USDC",
            required: true,
          },
          {
            key: "type",
            label: "Type",
            placeholder: "bank_account",
            required: true,
          },
          {
            key: "dest",
            label: "Destination",
            placeholder: "Bank account number",
          },
          { key: "amount", label: "Amount", placeholder: "100" },
        ],
      },
      {
        id: "sep6-transactions",
        method: "GET",
        path: "/transfer/transactions",
        description: "List transactions",
        params: [
          {
            key: "asset_code",
            label: "Asset Code",
            placeholder: "USDC",
            required: true,
          },
          {
            key: "account",
            label: "Stellar Account",
            placeholder: "G...",
            required: true,
          },
          { key: "limit", label: "Limit", placeholder: "10" },
        ],
      },
    ],
  },
  {
    id: "sep10",
    name: "SEP-10",
    tag: "Auth",
    color: "green",
    description: "Challenge-response authentication for Stellar accounts",
    endpoints: [
      {
        id: "sep10-challenge",
        method: "GET",
        path: "/auth",
        description: "Get a challenge transaction",
        params: [
          {
            key: "account",
            label: "Stellar Account",
            placeholder: "G...",
            required: true,
          },
          {
            key: "memo",
            label: "Memo ID",
            placeholder: "optional integer memo",
          },
          {
            key: "home_domain",
            label: "Home Domain",
            placeholder: "example.com",
          },
        ],
      },
      {
        id: "sep10-token",
        method: "POST",
        path: "/auth",
        description: "Submit signed challenge → JWT",
        params: [
          {
            key: "transaction",
            label: "Signed XDR",
            placeholder: "base64-encoded XDR",
            required: true,
          },
        ],
      },
    ],
  },
  {
    id: "sep12",
    name: "SEP-12",
    tag: "KYC",
    color: "rose",
    description: "Collect & manage customer KYC information",
    endpoints: [
      {
        id: "sep12-get",
        method: "GET",
        path: "/kyc/customer",
        description: "Retrieve KYC status",
        params: [
          {
            key: "account",
            label: "Stellar Account",
            placeholder: "G...",
            required: true,
          },
          { key: "type", label: "Customer Type", placeholder: "sep31-sender" },
        ],
      },
      {
        id: "sep12-put",
        method: "PUT",
        path: "/kyc/customer",
        description: "Submit KYC data",
        params: [
          {
            key: "account",
            label: "Stellar Account",
            placeholder: "G...",
            required: true,
          },
          {
            key: "first_name",
            label: "First Name",
            placeholder: "John",
            required: true,
          },
          {
            key: "last_name",
            label: "Last Name",
            placeholder: "Doe",
            required: true,
          },
          {
            key: "email_address",
            label: "Email",
            placeholder: "john@example.com",
          },
        ],
      },
    ],
  },
  {
    id: "sep24",
    name: "SEP-24",
    tag: "Interactive",
    color: "violet",
    description: "Hosted deposit & withdrawal with interactive UI flow",
    endpoints: [
      {
        id: "sep24-info",
        method: "GET",
        path: "/sep24/info",
        description: "Get assets & config",
        params: [],
      },
      {
        id: "sep24-deposit",
        method: "POST",
        path: "/sep24/transactions/deposit/interactive",
        description: "Interactive deposit",
        params: [
          {
            key: "asset_code",
            label: "Asset Code",
            placeholder: "USDC",
            required: true,
          },
          {
            key: "account",
            label: "Stellar Account",
            placeholder: "G...",
            required: true,
          },
          { key: "amount", label: "Amount", placeholder: "100" },
        ],
      },
      {
        id: "sep24-withdraw",
        method: "POST",
        path: "/sep24/transactions/withdraw/interactive",
        description: "Interactive withdraw",
        params: [
          {
            key: "asset_code",
            label: "Asset Code",
            placeholder: "USDC",
            required: true,
          },
          {
            key: "account",
            label: "Stellar Account",
            placeholder: "G...",
            required: true,
          },
        ],
      },
      {
        id: "sep24-transaction",
        method: "GET",
        path: "/sep24/transaction",
        description: "Get single transaction",
        params: [
          {
            key: "id",
            label: "Transaction ID",
            placeholder: "uuid",
            required: true,
          },
        ],
      },
    ],
  },
  {
    id: "sep31",
    name: "SEP-31",
    tag: "Cross-Border",
    color: "orange",
    description: "Direct payment API for cross-border remittances",
    endpoints: [
      {
        id: "sep31-info",
        method: "GET",
        path: "/sep31/info",
        description: "Get receiving anchor info",
        params: [],
      },
      {
        id: "sep31-send",
        method: "POST",
        path: "/sep31/transactions",
        description: "Create transaction",
        params: [
          {
            key: "amount",
            label: "Amount",
            placeholder: "100",
            required: true,
          },
          {
            key: "asset_code",
            label: "Asset Code",
            placeholder: "USDC",
            required: true,
          },
          {
            key: "sender_id",
            label: "Sender KYC ID",
            placeholder: "uuid",
            required: true,
          },
          {
            key: "receiver_id",
            label: "Receiver KYC ID",
            placeholder: "uuid",
            required: true,
          },
        ],
      },
    ],
  },
  {
    id: "sep38",
    name: "SEP-38",
    tag: "Quotes",
    color: "teal",
    description: "Anchor RFQ API – exchange rate quotes",
    endpoints: [
      {
        id: "sep38-info",
        method: "GET",
        path: "/sep38/info",
        description: "Supported assets for quoting",
        params: [],
      },
      {
        id: "sep38-prices",
        method: "GET",
        path: "/sep38/prices",
        description: "Get indicative prices",
        params: [
          {
            key: "sell_asset",
            label: "Sell Asset",
            placeholder: "stellar:USDC:G...",
            required: true,
          },
          {
            key: "sell_amount",
            label: "Sell Amount",
            placeholder: "100",
            required: true,
          },
        ],
      },
      {
        id: "sep38-quote",
        method: "POST",
        path: "/sep38/quote",
        description: "Request firm quote",
        params: [
          {
            key: "sell_asset",
            label: "Sell Asset",
            placeholder: "stellar:USDC:G...",
            required: true,
          },
          {
            key: "sell_amount",
            label: "Sell Amount",
            placeholder: "100",
            required: true,
          },
          {
            key: "buy_asset",
            label: "Buy Asset",
            placeholder: "iso4217:USD",
            required: true,
          },
        ],
      },
    ],
  },
];

// ─── Color Palette ────────────────────────────────────────────────────────────
const SEP_HEX: Record<string, { neon: string; dim: string }> = {
  cyan: { neon: "#00e5ff", dim: "rgba(0,229,255,0.10)" },
  blue: { neon: "#4d8dff", dim: "rgba(77,141,255,0.10)" },
  green: { neon: "#00ff9d", dim: "rgba(0,255,157,0.10)" },
  rose: { neon: "#ff3670", dim: "rgba(255,54,112,0.10)" },
  violet: { neon: "#c44dff", dim: "rgba(196,77,255,0.10)" },
  orange: { neon: "#ff8c00", dim: "rgba(255,140,0,0.10)" },
  teal: { neon: "#00ffcc", dim: "rgba(0,255,204,0.10)" },
};

const METHOD_DARK: Record<HttpMethod, string> = {
  GET: "text-emerald-300 bg-emerald-500/15 border-emerald-500/40",
  POST: "text-amber-300   bg-amber-500/15   border-amber-500/40",
  PUT: "text-violet-300  bg-violet-500/15  border-violet-500/40",
};
const METHOD_LIGHT: Record<HttpMethod, string> = {
  GET: "text-emerald-700 bg-emerald-50 border-emerald-300",
  POST: "text-amber-700   bg-amber-50   border-amber-300",
  PUT: "text-violet-700  bg-violet-50  border-violet-300",
};

// ─── JSON Syntax Highlighter ──────────────────────────────────────────────────
function highlight(json: string, dark: boolean): string {
  const colors = dark
    ? {
        key: "#ff7eb3",
        str: "#7effc7",
        num: "#79d4fd",
        bool: "#ffcb6b",
        nil: "#546e7a",
      }
    : {
        key: "#b5004d",
        str: "#006b35",
        num: "#0050bb",
        bool: "#8a5000",
        nil: "#888",
      };
  return json
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(
      /("(\\u[\da-fA-F]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+-]?\d+)?)/g,
      (m) => {
        let c = colors.num;
        if (/^"/.test(m)) c = /:$/.test(m) ? colors.key : colors.str;
        else if (/true|false/.test(m)) c = colors.bool;
        else if (/null/.test(m)) c = colors.nil;
        return `<span style="color:${c}">${m}</span>`;
      },
    );
}

// ─── Icons ────────────────────────────────────────────────────────────────────
const SunIcon = () => (
  <svg
    className="w-3.5 h-3.5"
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <circle cx="12" cy="12" r="5" strokeWidth={2} />
    <path
      strokeLinecap="round"
      strokeWidth={2}
      d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"
    />
  </svg>
);
const MoonIcon = () => (
  <svg
    className="w-3.5 h-3.5"
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"
    />
  </svg>
);
const CopyIcon = () => (
  <svg
    className="w-3 h-3"
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2"
    />
  </svg>
);
const SendIcon = () => (
  <svg
    className="w-3.5 h-3.5"
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M13 10V3L4 14h7v7l9-11h-7z"
    />
  </svg>
);

// ─── Component ────────────────────────────────────────────────────────────────
export default function AnchorPlayground() {
  const [dark, setDark] = useState(true);
  const [domain, setDomain] = useState("testanchor.stellar.org");
  const [activeSEP, setActiveSEP] = useState<SEPProtocol>(SEP_PROTOCOLS[0]);
  const [activeEp, setActiveEp] = useState<Endpoint>(
    SEP_PROTOCOLS[0].endpoints[0],
  );
  const [params, setParams] = useState<Record<string, string>>({});
  const [jwt, setJwt] = useState("");
  const [response, setResponse] = useState<{
    data: unknown;
    status: number;
    time: number;
  } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [tab, setTab] = useState<"response" | "history">("response");
  const [copied, setCopied] = useState(false);
  const [tick, setTick] = useState(0);
  const responseRef = useRef<HTMLDivElement>(null);

  // Pulsing scan line for dark mode
  useEffect(() => {
    const id = setInterval(() => setTick((t) => (t + 1) % 200), 40);
    return () => clearInterval(id);
  }, []);

  const neon = SEP_HEX[activeSEP.color].neon;
  const neonDim = SEP_HEX[activeSEP.color].dim;

  const selectSEP = (sep: SEPProtocol) => {
    setActiveSEP(sep);
    setActiveEp(sep.endpoints[0]);
    setParams({});
    setResponse(null);
    setError(null);
  };
  const selectEp = (ep: Endpoint) => {
    setActiveEp(ep);
    setParams({});
    setResponse(null);
    setError(null);
  };

  const buildUrl = useCallback(() => {
    const base = `https://${domain.replace(/^https?:\/\//, "")}${activeEp.path}`;
    if (activeEp.method === "GET") {
      const qs = new URLSearchParams(
        Object.fromEntries(Object.entries(params).filter(([, v]) => v)),
      ).toString();
      return qs ? `${base}?${qs}` : base;
    }
    return base;
  }, [domain, activeEp, params]);

  const sendRequest = useCallback(async () => {
    setLoading(true);
    setError(null);
    setResponse(null);
    const url = buildUrl();
    const t0 = performance.now();
    const headers: Record<string, string> = { Accept: "application/json" };
    if (jwt) headers["Authorization"] = `Bearer ${jwt}`;
    let body: string | undefined;
    if (activeEp.method !== "GET") {
      headers["Content-Type"] = "application/json";
      body = JSON.stringify(
        Object.fromEntries(Object.entries(params).filter(([, v]) => v)),
      );
    }
    try {
      const res = await fetch(url, { method: activeEp.method, headers, body });
      const elapsed = Math.round(performance.now() - t0);
      const ct = res.headers.get("content-type") ?? "";
      const data = ct.includes("json")
        ? await res.json()
        : { _raw: await res.text() };
      setResponse({ data, status: res.status, time: elapsed });
      setTab("response");
      setHistory((h) => [
        {
          id: crypto.randomUUID(),
          time: new Date().toLocaleTimeString(),
          sep: activeSEP.name,
          endpoint: activeEp.path,
          method: activeEp.method,
          status: res.status,
          success: res.ok,
        },
        ...h.slice(0, 19),
      ]);
    } catch (err) {
      const elapsed = Math.round(performance.now() - t0);
      const msg = err instanceof Error ? err.message : "Unknown error";
      setError(msg);
      setResponse({ data: { error: msg }, status: 0, time: elapsed });
      setTab("response");
      setHistory((h) => [
        {
          id: crypto.randomUUID(),
          time: new Date().toLocaleTimeString(),
          sep: activeSEP.name,
          endpoint: activeEp.path,
          method: activeEp.method,
          status: null,
          success: false,
        },
        ...h.slice(0, 19),
      ]);
    } finally {
      setLoading(false);
      setTimeout(
        () => responseRef.current?.scrollIntoView({ behavior: "smooth" }),
        80,
      );
    }
  }, [buildUrl, jwt, activeEp, params, activeSEP]);

  const copyResponse = () => {
    if (!response) return;
    navigator.clipboard.writeText(JSON.stringify(response.data, null, 2));
    setCopied(true);
    setTimeout(() => setCopied(false), 1600);
  };

  // ── Derived theme tokens ──
  const D = dark;
  const bg = D ? "#050810" : "#eef2fa";
  const surfaceBg = D ? "#080c18" : "#ffffff";
  const panelBg = D ? "rgba(9,13,26,0.85)" : "rgba(248,250,255,0.9)";
  const borderCol = D ? "#18243d" : "#ccd4e8";
  const textCol = D ? "#dde6f5" : "#1e2a45";
  const mutedCol = D ? "#3a5070" : "#8899bb";
  const inputBg = D ? "#0a1020" : "#ffffff";
  const inputBord = D ? "#1c2d4a" : "#c0ccdf";
  const codeBg = D ? "#020408" : "#f4f7ff";

  return (
    <div
      style={{
        fontFamily: "'JetBrains Mono','Fira Code',monospace",
        background: bg,
        color: textCol,
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        position: "relative",
        overflow: "hidden",
      }}
    >
      {/* ── Grid bg ── */}
      <div
        style={{
          position: "fixed",
          inset: 0,
          pointerEvents: "none",
          backgroundImage: `linear-gradient(${D ? "rgba(80,120,255,0.04)" : "rgba(60,100,200,0.05)"} 1px, transparent 1px), linear-gradient(90deg, ${D ? "rgba(80,120,255,0.04)" : "rgba(60,100,200,0.05)"} 1px, transparent 1px)`,
          backgroundSize: "48px 48px",
        }}
      />

      {/* ── Ambient glows ── */}
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

      {/* ── Scanline (dark only) ── */}
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
            style={{
              position: "absolute",
              left: 0,
              right: 0,
              height: 80,
              top: `${(tick / 200) * 120 - 10}%`,
              background: `linear-gradient(transparent,${neon}06,transparent)`,
              transition: "top 0.04s linear",
            }}
          />
        </div>
      )}

      {/* ── Corner brackets (dark) ── */}
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

      {/* ═══ HEADER ═══ */}
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
                textTransform: "uppercase",
              }}
            >
              Anchor // Playground
            </div>
            <div
              style={{
                fontSize: 9,
                letterSpacing: "0.22em",
                color: mutedCol,
                textTransform: "uppercase",
              }}
            >
              Stellar SEP Protocol Tester
            </div>
          </div>
        </div>

        {/* Right */}
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          {/* Status pill */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              padding: "6px 12px",
              borderRadius: 6,
              border: `1px solid ${borderCol}`,
              background: D ? "rgba(0,0,0,0.3)" : "rgba(240,244,255,0.6)",
              fontSize: 10,
              color: mutedCol,
            }}
          >
            <span
              style={{
                width: 7,
                height: 7,
                borderRadius: "50%",
                background: "#22c55e",
                boxShadow: "0 0 8px #22c55e",
                display: "inline-block",
                animation: "pulse 2s infinite",
              }}
            />
            <span
              style={{
                color: "#22c55e",
                fontWeight: 700,
                letterSpacing: "0.15em",
              }}
            >
              LIVE
            </span>
            <span style={{ color: borderCol }}>·</span>
            <span
              style={{
                maxWidth: 160,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {domain || "—"}
            </span>
          </div>

          {/* Dark/Light toggle */}
          <button
            onClick={() => setDark((d) => !d)}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 7,
              padding: "7px 14px",
              borderRadius: 6,
              fontSize: 10,
              fontWeight: 700,
              letterSpacing: "0.15em",
              textTransform: "uppercase",
              cursor: "pointer",
              transition: "all 0.25s",
              border: `1px solid ${D ? neon : borderCol}`,
              background: D ? neonDim : "rgba(230,236,250,0.7)",
              color: D ? neon : "#4466aa",
              boxShadow: D ? `0 0 16px ${neon}28` : "none",
              fontFamily: "inherit",
            }}
          >
            {D ? <SunIcon /> : <MoonIcon />}
            {D ? "Light Mode" : "Dark Mode"}
          </button>
        </div>
      </header>

      {/* ═══ BODY ═══ */}
      <div
        style={{
          position: "relative",
          zIndex: 10,
          flex: 1,
          display: "flex",
          overflow: "hidden",
        }}
      >
        {/* ═══ SIDEBAR ═══ */}
        <aside
          style={{
            width: 300,
            flexShrink: 0,
            display: "flex",
            flexDirection: "column",
            borderRight: `1px solid ${borderCol}`,
            background: panelBg,
            backdropFilter: "blur(12px)",
            overflow: "hidden",
          }}
        >
          {/* Domain */}
          <div
            style={{
              padding: "16px 16px 14px",
              borderBottom: `1px solid ${borderCol}`,
            }}
          >
            <div
              style={{
                fontSize: 9,
                fontWeight: 700,
                letterSpacing: "0.2em",
                color: mutedCol,
                textTransform: "uppercase",
                marginBottom: 8,
              }}
            >
              ◈ Anchor Domain
            </div>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                border: `1px solid ${inputBord}`,
                borderRadius: 6,
                overflow: "hidden",
                background: inputBg,
              }}
            >
              <span
                style={{
                  padding: "9px 10px",
                  fontSize: 10,
                  fontWeight: 700,
                  letterSpacing: "0.1em",
                  color: neon,
                  borderRight: `1px solid ${inputBord}`,
                  background: D ? "rgba(0,0,0,0.35)" : "rgba(230,236,250,0.5)",
                  whiteSpace: "nowrap",
                }}
              >
                https://
              </span>
              <input
                style={{
                  flex: 1,
                  padding: "9px 10px",
                  fontSize: 11,
                  background: "transparent",
                  border: "none",
                  outline: "none",
                  color: textCol,
                  fontFamily: "inherit",
                }}
                placeholder="testanchor.stellar.org"
                value={domain}
                onChange={(e) => setDomain(e.target.value)}
              />
            </div>
          </div>

          {/* JWT */}
          <div
            style={{
              padding: "12px 16px 14px",
              borderBottom: `1px solid ${borderCol}`,
            }}
          >
            <div
              style={{
                fontSize: 9,
                fontWeight: 700,
                letterSpacing: "0.2em",
                color: mutedCol,
                textTransform: "uppercase",
                marginBottom: 8,
              }}
            >
              ◈ JWT Token{" "}
              <span
                style={{ fontWeight: 400, color: D ? "#23334d" : "#aab8cc" }}
              >
                (optional)
              </span>
            </div>
            <input
              style={{
                width: "100%",
                padding: "8px 10px",
                fontSize: 11,
                background: inputBg,
                border: `1px solid ${inputBord}`,
                borderRadius: 6,
                outline: "none",
                color: textCol,
                fontFamily: "inherit",
                boxSizing: "border-box",
              }}
              placeholder="eyJhbGci..."
              value={jwt}
              onChange={(e) => setJwt(e.target.value)}
            />
          </div>

          {/* SEP selector */}
          <div
            style={{
              padding: "12px 16px 14px",
              borderBottom: `1px solid ${borderCol}`,
            }}
          >
            <div
              style={{
                fontSize: 9,
                fontWeight: 700,
                letterSpacing: "0.2em",
                color: mutedCol,
                textTransform: "uppercase",
                marginBottom: 10,
              }}
            >
              ◈ SEP Protocol
            </div>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(4, 1fr)",
                gap: 6,
              }}
            >
              {SEP_PROTOCOLS.map((sep) => {
                const isActive = sep.id === activeSEP.id;
                const sc = SEP_HEX[sep.color];
                return (
                  <button
                    key={sep.id}
                    onClick={() => selectSEP(sep)}
                    style={{
                      position: "relative",
                      padding: "8px 4px",
                      borderRadius: 6,
                      fontSize: 9,
                      fontWeight: 700,
                      letterSpacing: "0.12em",
                      textTransform: "uppercase",
                      cursor: "pointer",
                      transition: "all 0.2s",
                      fontFamily: "inherit",
                      border: `1px solid ${isActive ? sc.neon : D ? "#18243d" : "#ccd4e8"}`,
                      background: isActive
                        ? sc.dim
                        : D
                          ? "rgba(0,0,0,0.25)"
                          : "rgba(230,236,250,0.5)",
                      color: isActive ? sc.neon : D ? "#2a3d5a" : "#8899bb",
                      boxShadow: isActive
                        ? `0 0 14px ${sc.neon}28, inset 0 0 10px ${sc.neon}06`
                        : "none",
                      transform: isActive ? "scale(1.05)" : "scale(1)",
                    }}
                  >
                    {isActive && (
                      <>
                        <div
                          style={{
                            position: "absolute",
                            top: 0,
                            left: 0,
                            width: 7,
                            height: 7,
                            borderTop: `1.5px solid ${sc.neon}`,
                            borderLeft: `1.5px solid ${sc.neon}`,
                          }}
                        />
                        <div
                          style={{
                            position: "absolute",
                            bottom: 0,
                            right: 0,
                            width: 7,
                            height: 7,
                            borderBottom: `1.5px solid ${sc.neon}`,
                            borderRight: `1.5px solid ${sc.neon}`,
                          }}
                        />
                      </>
                    )}
                    {sep.name}
                  </button>
                );
              })}
            </div>
            {/* SEP info chip */}
            <div
              style={{
                marginTop: 10,
                padding: "8px 10px",
                borderRadius: 6,
                fontSize: 10,
                lineHeight: 1.5,
                border: `1px solid ${neon}28`,
                background: neonDim,
                color: neon,
                transition: "all 0.35s",
              }}
            >
              <strong>{activeSEP.tag}</strong> —{" "}
              <span style={{ opacity: 0.7 }}>{activeSEP.description}</span>
            </div>
          </div>

          {/* Endpoints */}
          <div
            style={{
              padding: "12px 16px",
              borderBottom: `1px solid ${borderCol}`,
              flex: 1,
              overflowY: "auto",
            }}
          >
            <div
              style={{
                fontSize: 9,
                fontWeight: 700,
                letterSpacing: "0.2em",
                color: mutedCol,
                textTransform: "uppercase",
                marginBottom: 10,
              }}
            >
              ◈ Endpoint
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              {activeSEP.endpoints.map((ep) => {
                const isActive = ep.id === activeEp.id;
                const methodStyle = D
                  ? METHOD_DARK[ep.method]
                  : METHOD_LIGHT[ep.method];
                const [mtColor, mtBg, mtBorder] = methodStyle.split(" "); // use tailwind className approach
                return (
                  <button
                    key={ep.id}
                    onClick={() => selectEp(ep)}
                    style={{
                      position: "relative",
                      textAlign: "left",
                      padding: "9px 10px 9px 14px",
                      borderRadius: 6,
                      cursor: "pointer",
                      transition: "all 0.15s",
                      fontFamily: "inherit",
                      border: `1px solid ${isActive ? neon + "55" : D ? "#18243d" : "#ccd4e8"}`,
                      background: isActive
                        ? neonDim
                        : D
                          ? "rgba(0,0,0,0.18)"
                          : "rgba(230,236,250,0.4)",
                    }}
                  >
                    {isActive && (
                      <div
                        style={{
                          position: "absolute",
                          left: 0,
                          top: "50%",
                          transform: "translateY(-50%)",
                          width: 3,
                          height: 18,
                          borderRadius: 2,
                          background: neon,
                          boxShadow: `0 0 10px ${neon}`,
                        }}
                      />
                    )}
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 6,
                        marginBottom: 4,
                      }}
                    >
                      <span
                        style={{
                          fontSize: 8,
                          fontWeight: 700,
                          padding: "2px 6px",
                          borderRadius: 4,
                          border: `1px solid`,
                          color: isActive
                            ? ep.method === "GET"
                              ? "#6ee7b7"
                              : ep.method === "POST"
                                ? "#fcd34d"
                                : "#c4b5fd"
                            : mutedCol,
                          borderColor: isActive
                            ? ep.method === "GET"
                              ? "#6ee7b780"
                              : ep.method === "POST"
                                ? "#fcd34d80"
                                : "#c4b5fd80"
                            : borderCol,
                          background: isActive
                            ? ep.method === "GET"
                              ? "rgba(110,231,183,0.12)"
                              : ep.method === "POST"
                                ? "rgba(252,211,77,0.12)"
                                : "rgba(196,181,253,0.12)"
                            : D
                              ? "rgba(0,0,0,0.3)"
                              : "rgba(220,228,244,0.6)",
                        }}
                      >
                        {ep.method}
                      </span>
                      <span
                        style={{
                          fontSize: 10,
                          fontWeight: 600,
                          color: isActive ? neon : D ? "#3a5070" : "#8899bb",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                          maxWidth: 160,
                        }}
                      >
                        {ep.path}
                      </span>
                    </div>
                    <div
                      style={{
                        fontSize: 9,
                        color: D ? "#28384e" : "#aab8cc",
                        paddingLeft: 32,
                        lineHeight: 1.4,
                      }}
                    >
                      {ep.description}
                    </div>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Params */}
          {activeEp.params.length > 0 && (
            <div
              style={{
                padding: "12px 16px",
                borderBottom: `1px solid ${borderCol}`,
                maxHeight: 210,
                overflowY: "auto",
              }}
            >
              <div
                style={{
                  fontSize: 9,
                  fontWeight: 700,
                  letterSpacing: "0.2em",
                  color: mutedCol,
                  textTransform: "uppercase",
                  marginBottom: 10,
                }}
              >
                ◈ Parameters
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {activeEp.params.map((p) => (
                  <div key={p.key}>
                    <div
                      style={{ fontSize: 9, color: mutedCol, marginBottom: 4 }}
                    >
                      {p.label}
                      {p.required && (
                        <span style={{ color: "#ff5577" }}> *</span>
                      )}
                    </div>
                    <input
                      style={{
                        width: "100%",
                        padding: "7px 10px",
                        fontSize: 11,
                        background: inputBg,
                        border: `1px solid ${inputBord}`,
                        borderRadius: 5,
                        outline: "none",
                        color: textCol,
                        fontFamily: "inherit",
                        boxSizing: "border-box",
                        transition: "border-color 0.2s",
                      }}
                      onFocus={(e) => {
                        e.currentTarget.style.borderColor = neon + "70";
                        e.currentTarget.style.boxShadow = `0 0 0 2px ${neon}18`;
                      }}
                      onBlur={(e) => {
                        e.currentTarget.style.borderColor = inputBord;
                        e.currentTarget.style.boxShadow = "none";
                      }}
                      placeholder={p.placeholder}
                      value={params[p.key] ?? ""}
                      onChange={(e) =>
                        setParams((prev) => ({
                          ...prev,
                          [p.key]: e.target.value,
                        }))
                      }
                    />
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Send btn */}
          <div style={{ padding: 16 }}>
            <button
              onClick={sendRequest}
              disabled={loading || !domain}
              style={{
                width: "100%",
                padding: "13px 0",
                borderRadius: 8,
                fontSize: 11,
                fontWeight: 700,
                letterSpacing: "0.2em",
                textTransform: "uppercase",
                fontFamily: "inherit",
                cursor: loading || !domain ? "not-allowed" : "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: 8,
                position: "relative",
                overflow: "hidden",
                transition: "all 0.25s",
                border: `1px solid ${loading || !domain ? (D ? "#18243d" : "#ccd4e8") : neon}`,
                background:
                  loading || !domain
                    ? D
                      ? "rgba(0,0,0,0.25)"
                      : "rgba(220,228,244,0.4)"
                    : `linear-gradient(135deg, ${neonDim}, ${neon}18)`,
                color: loading || !domain ? (D ? "#23334d" : "#aab8cc") : neon,
                boxShadow:
                  !loading && domain
                    ? `0 0 22px ${neon}35, inset 0 0 18px ${neon}06`
                    : "none",
              }}
            >
              {!loading && domain && (
                <>
                  <div
                    style={{
                      position: "absolute",
                      top: 0,
                      left: 0,
                      width: 12,
                      height: 12,
                      borderTop: `2px solid ${neon}`,
                      borderLeft: `2px solid ${neon}`,
                    }}
                  />
                  <div
                    style={{
                      position: "absolute",
                      bottom: 0,
                      right: 0,
                      width: 12,
                      height: 12,
                      borderBottom: `2px solid ${neon}`,
                      borderRight: `2px solid ${neon}`,
                    }}
                  />
                </>
              )}
              {loading ? (
                <>
                  <svg
                    style={{
                      width: 14,
                      height: 14,
                      animation: "spin 0.7s linear infinite",
                    }}
                    fill="none"
                    viewBox="0 0 24 24"
                  >
                    <circle
                      style={{ opacity: 0.25 }}
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      strokeWidth="3"
                    />
                    <path
                      style={{ opacity: 0.8 }}
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                    />
                  </svg>
                  Transmitting...
                </>
              ) : (
                <>
                  <SendIcon />
                  Execute Request
                </>
              )}
            </button>
          </div>
        </aside>

        {/* ═══ MAIN PANEL ═══ */}
        <main
          style={{
            flex: 1,
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}
        >
          {/* URL bar */}
          <div
            style={{
              flexShrink: 0,
              padding: "10px 20px",
              borderBottom: `1px solid ${borderCol}`,
              background: D ? "rgba(5,8,16,0.7)" : "rgba(240,244,255,0.7)",
              display: "flex",
              alignItems: "center",
              gap: 10,
            }}
          >
            <span
              style={{
                fontSize: 9,
                fontWeight: 700,
                padding: "3px 8px",
                borderRadius: 4,
                border: "1px solid",
                flexShrink: 0,
                color:
                  activeEp.method === "GET"
                    ? D
                      ? "#6ee7b7"
                      : "#2a8a58"
                    : activeEp.method === "POST"
                      ? D
                        ? "#fcd34d"
                        : "#9a6800"
                      : D
                        ? "#c4b5fd"
                        : "#6600cc",
                borderColor:
                  activeEp.method === "GET"
                    ? D
                      ? "#6ee7b760"
                      : "#a7d8b8"
                    : activeEp.method === "POST"
                      ? D
                        ? "#fcd34d60"
                        : "#d4b060"
                      : D
                        ? "#c4b5fd60"
                        : "#b090e8",
                background:
                  activeEp.method === "GET"
                    ? D
                      ? "rgba(110,231,183,0.12)"
                      : "#f0faf5"
                    : activeEp.method === "POST"
                      ? D
                        ? "rgba(252,211,77,0.12)"
                        : "#fffbf0"
                      : D
                        ? "rgba(196,181,253,0.12)"
                        : "#f8f0ff",
              }}
            >
              {activeEp.method}
            </span>
            <div
              style={{
                flex: 1,
                fontSize: 11,
                padding: "7px 12px",
                borderRadius: 6,
                border: `1px solid ${borderCol}`,
                background: D ? "rgba(0,0,0,0.4)" : "rgba(240,244,255,0.8)",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              <span style={{ color: mutedCol }}>https://</span>
              <span style={{ color: neon }}>{domain}</span>
              <span style={{ color: D ? "#8899bb" : "#445577" }}>
                {activeEp.path}
              </span>
              {activeEp.method === "GET" &&
                Object.entries(params).some(([, v]) => v) && (
                  <>
                    <span style={{ color: mutedCol }}>?</span>
                    <span style={{ color: D ? "#79d4fd" : "#0055aa" }}>
                      {Object.entries(params)
                        .filter(([, v]) => v)
                        .map(([k, v]) => `${k}=${v}`)
                        .join("&")}
                    </span>
                  </>
                )}
            </div>
            {response && (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  flexShrink: 0,
                }}
              >
                <span
                  style={{
                    fontSize: 10,
                    fontWeight: 700,
                    padding: "3px 8px",
                    borderRadius: 4,
                    border: "1px solid",
                    color:
                      response.status >= 200 && response.status < 300
                        ? D
                          ? "#6ee7b7"
                          : "#2a8a58"
                        : D
                          ? "#ff7eb3"
                          : "#c0005a",
                    borderColor:
                      response.status >= 200 && response.status < 300
                        ? D
                          ? "#6ee7b760"
                          : "#a7d8b8"
                        : D
                          ? "#ff7eb360"
                          : "#e0a0b8",
                    background:
                      response.status >= 200 && response.status < 300
                        ? D
                          ? "rgba(110,231,183,0.12)"
                          : "#f0faf5"
                        : D
                          ? "rgba(255,126,179,0.12)"
                          : "#fff0f5",
                  }}
                >
                  {response.status >= 200 && response.status < 300 ? "✓" : "✕"}{" "}
                  {response.status || "ERR"}
                </span>
                <span style={{ fontSize: 10, color: mutedCol }}>
                  {response.time}ms
                </span>
              </div>
            )}
          </div>

          {/* Tabs */}
          <div
            style={{
              flexShrink: 0,
              display: "flex",
              alignItems: "center",
              padding: "0 20px",
              borderBottom: `1px solid ${borderCol}`,
              background: D ? "rgba(5,8,16,0.5)" : "rgba(240,244,255,0.5)",
            }}
          >
            {(["response", "history"] as const).map((t) => (
              <button
                key={t}
                onClick={() => setTab(t)}
                style={{
                  position: "relative",
                  padding: "11px 16px",
                  fontSize: 9,
                  fontWeight: 700,
                  letterSpacing: "0.2em",
                  textTransform: "uppercase",
                  fontFamily: "inherit",
                  cursor: "pointer",
                  border: "none",
                  background: "transparent",
                  color: tab === t ? neon : mutedCol,
                  transition: "color 0.2s",
                }}
              >
                {t === "response" ? "◉ RESPONSE" : "◎ HISTORY"}
                {t === "history" && history.length > 0 && (
                  <span
                    style={{
                      marginLeft: 6,
                      fontSize: 8,
                      padding: "1px 5px",
                      borderRadius: 10,
                      background: D ? "#18243d" : "#dde6f5",
                      color: mutedCol,
                    }}
                  >
                    {history.length}
                  </span>
                )}
                {tab === t && (
                  <div
                    style={{
                      position: "absolute",
                      bottom: 0,
                      left: 0,
                      right: 0,
                      height: 2,
                      background: neon,
                      boxShadow: `0 0 8px ${neon}`,
                    }}
                  />
                )}
              </button>
            ))}
          </div>

          {/* Content */}
          <div
            ref={responseRef}
            style={{ flex: 1, overflowY: "auto", padding: 20 }}
          >
            {tab === "response" && (
              <>
                {/* Empty state */}
                {!response && !loading && (
                  <div
                    style={{
                      height: "100%",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                    }}
                  >
                    <div style={{ textAlign: "center" }}>
                      <div
                        style={{
                          fontSize: 56,
                          opacity: 0.08,
                          marginBottom: 16,
                        }}
                      >
                        ⚓
                      </div>
                      <div
                        style={{
                          fontSize: 12,
                          letterSpacing: "0.2em",
                          color: mutedCol,
                          textTransform: "uppercase",
                        }}
                      >
                        Awaiting Transmission
                      </div>
                      <div
                        style={{
                          fontSize: 10,
                          color: D ? "#1c2c42" : "#ccd4e8",
                          marginTop: 6,
                          letterSpacing: "0.12em",
                        }}
                      >
                        Configure → Execute
                      </div>
                      <div
                        style={{
                          display: "flex",
                          justifyContent: "center",
                          gap: 8,
                          marginTop: 20,
                        }}
                      >
                        {[0, 1, 2].map((i) => (
                          <div
                            key={i}
                            style={{
                              width: 6,
                              height: 6,
                              borderRadius: "50%",
                              background: neon,
                              opacity: 0.3,
                              animation: `pulse ${1 + i * 0.3}s infinite`,
                            }}
                          />
                        ))}
                      </div>
                    </div>
                  </div>
                )}

                {/* Loading state with skeleton loaders */}
                {loading && (
                  <div
                    role="status"
                    aria-busy="true"
                    aria-label="Loading anchor data"
                    style={{
                      height: "100%",
                      padding: "20px",
                    }}
                  >
                    {/* Asset list skeleton */}
                    <div style={{ marginBottom: 24 }}>
                      <div
                        style={{
                          fontSize: 9,
                          fontWeight: 700,
                          letterSpacing: "0.2em",
                          color: mutedCol,
                          textTransform: "uppercase",
                          marginBottom: 12,
                        }}
                      >
                        Loading Assets...
                      </div>
                      {[1, 2, 3].map((i) => (
                        <div
                          key={i}
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 12,
                            padding: "12px 14px",
                            marginBottom: 8,
                            borderRadius: 8,
                            background: D ? "rgba(255,255,255,0.03)" : "rgba(0,0,0,0.03)",
                          }}
                        >
                          <div
                            style={{
                              width: 40,
                              height: 40,
                              borderRadius: "50%",
                              background: `linear-gradient(90deg, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 0%, ${D ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.10)"} 50%, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 100%)`,
                              backgroundSize: "200% 100%",
                              animation: "skeleton-shimmer 1.5s ease-in-out infinite",
                            }}
                          />
                          <div style={{ flex: 1 }}>
                            <div
                              style={{
                                width: `${60 + Math.random() * 30}%`,
                                height: 14,
                                marginBottom: 6,
                                borderRadius: 4,
                                background: `linear-gradient(90deg, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 0%, ${D ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.10)"} 50%, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 100%)`,
                                backgroundSize: "200% 100%",
                                animation: "skeleton-shimmer 1.5s ease-in-out infinite",
                              }}
                            />
                            <div
                              style={{
                                width: `${40 + Math.random() * 20}%`,
                                height: 12,
                                borderRadius: 4,
                                background: `linear-gradient(90deg, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 0%, ${D ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.10)"} 50%, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 100%)`,
                                backgroundSize: "200% 100%",
                                animation: "skeleton-shimmer 1.5s ease-in-out infinite",
                              }}
                            />
                          </div>
                        </div>
                      ))}
                    </div>

                    {/* Fee table skeleton */}
                    <div style={{ marginBottom: 24 }}>
                      <div
                        style={{
                          fontSize: 9,
                          fontWeight: 700,
                          letterSpacing: "0.2em",
                          color: mutedCol,
                          textTransform: "uppercase",
                          marginBottom: 12,
                        }}
                      >
                        Loading Fees...
                      </div>
                      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                        {/* Header */}
                        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12 }}>
                          {[1, 2, 3].map((i) => (
                            <div
                              key={i}
                              style={{
                                height: 16,
                                width: "80%",
                                borderRadius: 4,
                                background: `linear-gradient(90deg, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 0%, ${D ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.10)"} 50%, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 100%)`,
                                backgroundSize: "200% 100%",
                                animation: "skeleton-shimmer 1.5s ease-in-out infinite",
                              }}
                            />
                          ))}
                        </div>
                        {/* Rows */}
                        {[1, 2, 3].map((i) => (
                          <div key={i} style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12 }}>
                            {[1, 2, 3].map((j) => (
                              <div
                                key={j}
                                style={{
                                  height: 14,
                                  width: `${50 + Math.random() * 40}%`,
                                  borderRadius: 4,
                                  background: `linear-gradient(90deg, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 0%, ${D ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.10)"} 50%, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 100%)`,
                                  backgroundSize: "200% 100%",
                                  animation: "skeleton-shimmer 1.5s ease-in-out infinite",
                                }}
                              />
                            ))}
                          </div>
                        ))}
                      </div>
                    </div>

                    {/* Limits skeleton */}
                    <div>
                      <div
                        style={{
                          fontSize: 9,
                          fontWeight: 700,
                          letterSpacing: "0.2em",
                          color: mutedCol,
                          textTransform: "uppercase",
                          marginBottom: 12,
                        }}
                      >
                        Loading Limits...
                      </div>
                      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
                        {[1, 2].map((i) => (
                          <div key={i} style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                            <div
                              style={{
                                width: "40%",
                                height: 14,
                                borderRadius: 4,
                                background: `linear-gradient(90deg, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 0%, ${D ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.10)"} 50%, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 100%)`,
                                backgroundSize: "200% 100%",
                                animation: "skeleton-shimmer 1.5s ease-in-out infinite",
                              }}
                            />
                            <div
                              style={{
                                width: "100%",
                                height: 8,
                                borderRadius: 4,
                                background: `linear-gradient(90deg, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 0%, ${D ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.10)"} 50%, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 100%)`,
                                backgroundSize: "200% 100%",
                                animation: "skeleton-shimmer 1.5s ease-in-out infinite",
                              }}
                            />
                            <div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
                              <div
                                style={{
                                  width: "30%",
                                  height: 12,
                                  borderRadius: 4,
                                  background: `linear-gradient(90deg, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 0%, ${D ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.10)"} 50%, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 100%)`,
                                  backgroundSize: "200% 100%",
                                  animation: "skeleton-shimmer 1.5s ease-in-out infinite",
                                }}
                              />
                              <div
                                style={{
                                  width: "30%",
                                  height: 12,
                                  borderRadius: 4,
                                  background: `linear-gradient(90deg, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 0%, ${D ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.10)"} 50%, ${D ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.06)"} 100%)`,
                                  backgroundSize: "200% 100%",
                                  animation: "skeleton-shimmer 1.5s ease-in-out infinite",
                                }}
                              />
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>

                    {/* Screen reader text */}
                    <span style={{ position: "absolute", width: 1, height: 1, overflow: "hidden", clip: "rect(0,0,0,0)" }}>
                      Loading anchor data
                    </span>
                  </div>
                )}

                {/* Response */}
                {response && !loading && (
                  <div>
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        marginBottom: 14,
                      }}
                    >
                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 10,
                        }}
                      >
                        <span
                          style={{
                            fontSize: 10,
                            fontWeight: 700,
                            padding: "3px 10px",
                            borderRadius: 4,
                            border: "1px solid",
                            color:
                              response.status >= 200 && response.status < 300
                                ? D
                                  ? "#6ee7b7"
                                  : "#2a8a58"
                                : D
                                  ? "#ff7eb3"
                                  : "#c0005a",
                            borderColor:
                              response.status >= 200 && response.status < 300
                                ? D
                                  ? "#6ee7b760"
                                  : "#a7d8b8"
                                : D
                                  ? "#ff7eb360"
                                  : "#e0a0b8",
                            background:
                              response.status >= 200 && response.status < 300
                                ? D
                                  ? "rgba(110,231,183,0.12)"
                                  : "#f0faf5"
                                : D
                                  ? "rgba(255,126,179,0.12)"
                                  : "#fff0f5",
                          }}
                        >
                          {response.status >= 200 && response.status < 300
                            ? "✓"
                            : "✕"}{" "}
                          {response.status || "ERROR"}
                        </span>
                        <span style={{ fontSize: 10, color: mutedCol }}>
                          {response.time}ms
                        </span>
                        {error && (
                          <span style={{ fontSize: 10, color: "#ff5577" }}>
                            Network Error
                          </span>
                        )}
                      </div>
                      <button
                        onClick={copyResponse}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 6,
                          fontSize: 10,
                          fontWeight: 700,
                          letterSpacing: "0.12em",
                          textTransform: "uppercase",
                          padding: "6px 12px",
                          borderRadius: 5,
                          cursor: "pointer",
                          fontFamily: "inherit",
                          transition: "all 0.15s",
                          border: `1px solid ${copied ? neon : D ? "#18243d" : "#ccd4e8"}`,
                          color: copied ? neon : mutedCol,
                          background: copied ? neonDim : "transparent",
                          boxShadow: copied ? `0 0 12px ${neon}30` : "none",
                        }}
                      >
                        <CopyIcon />
                        {copied ? "Copied!" : "Copy"}
                      </button>
                    </div>

                    {/* JSON panel */}
                    <div
                      style={{
                        borderRadius: 8,
                        overflow: "hidden",
                        border: `1px solid ${neon}28`,
                        boxShadow: D
                          ? `0 0 50px ${neon}06`
                          : "0 4px 24px rgba(0,0,0,0.06)",
                      }}
                    >
                      {/* Titlebar */}
                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 10,
                          padding: "9px 16px",
                          background: D ? neonDim : "rgba(236,242,255,0.8)",
                          borderBottom: `1px solid ${neon}20`,
                        }}
                      >
                        <div style={{ display: "flex", gap: 6 }}>
                          {["#ff5f57", "#febc2e", "#28c840"].map((c) => (
                            <div
                              key={c}
                              style={{
                                width: 10,
                                height: 10,
                                borderRadius: "50%",
                                background: c,
                                opacity: 0.7,
                              }}
                            />
                          ))}
                        </div>
                        <span
                          style={{
                            fontSize: 9,
                            fontWeight: 700,
                            letterSpacing: "0.2em",
                            textTransform: "uppercase",
                            color: neon,
                          }}
                        >
                          JSON Response
                        </span>
                        <div
                          style={{
                            marginLeft: "auto",
                            display: "flex",
                            alignItems: "center",
                            gap: 6,
                          }}
                        >
                          <div
                            style={{
                              width: 6,
                              height: 6,
                              borderRadius: "50%",
                              background: neon,
                              animation: "pulse 2s infinite",
                            }}
                          />
                          <span style={{ fontSize: 9, color: mutedCol }}>
                            PARSED
                          </span>
                        </div>
                      </div>
                      <pre
                        style={{
                          margin: 0,
                          padding: "18px 20px",
                          fontSize: 11,
                          lineHeight: 1.7,
                          overflowX: "auto",
                          maxHeight: "60vh",
                          background: codeBg,
                          color: D ? "#6688aa" : "#445577",
                        }}
                        dangerouslySetInnerHTML={{
                          __html: highlight(
                            JSON.stringify(response.data, null, 2),
                            D,
                          ),
                        }}
                      />
                    </div>
                  </div>
                )}
              </>
            )}

            {tab === "history" && (
              <div>
                {history.length === 0 ? (
                  <div style={{ padding: "80px 0", textAlign: "center" }}>
                    <div
                      style={{ fontSize: 32, opacity: 0.1, marginBottom: 12 }}
                    >
                      ◎
                    </div>
                    <div
                      style={{
                        fontSize: 11,
                        letterSpacing: "0.18em",
                        color: mutedCol,
                        textTransform: "uppercase",
                      }}
                    >
                      No History Yet
                    </div>
                  </div>
                ) : (
                  <div
                    style={{ display: "flex", flexDirection: "column", gap: 6 }}
                  >
                    {history.map((entry, i) => {
                      const sc =
                        SEP_HEX[
                          SEP_PROTOCOLS.find((s) => s.name === entry.sep)
                            ?.color ?? "cyan"
                        ];
                      return (
                        <div
                          key={entry.id}
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 10,
                            padding: "10px 14px",
                            borderRadius: 7,
                            opacity: 1 - i * 0.03,
                            transition: "all 0.15s",
                            border: `1px solid ${entry.success ? (D ? "#18243d" : "#ccd4e8") : "rgba(255,51,119,0.25)"}`,
                            background: entry.success
                              ? D
                                ? "rgba(0,0,0,0.2)"
                                : "rgba(230,236,250,0.4)"
                              : D
                                ? "rgba(255,51,119,0.04)"
                                : "rgba(255,51,119,0.03)",
                          }}
                        >
                          <span
                            style={{
                              fontSize: 9,
                              color: mutedCol,
                              width: 60,
                              flexShrink: 0,
                            }}
                          >
                            {entry.time}
                          </span>
                          <span
                            style={{
                              fontSize: 8,
                              fontWeight: 700,
                              padding: "2px 7px",
                              borderRadius: 4,
                              border: "1px solid",
                              flexShrink: 0,
                              color:
                                entry.method === "GET"
                                  ? D
                                    ? "#6ee7b7"
                                    : "#2a8a58"
                                  : entry.method === "POST"
                                    ? D
                                      ? "#fcd34d"
                                      : "#9a6800"
                                    : D
                                      ? "#c4b5fd"
                                      : "#6600cc",
                              borderColor:
                                entry.method === "GET"
                                  ? D
                                    ? "#6ee7b750"
                                    : "#a7d8b8"
                                  : entry.method === "POST"
                                    ? D
                                      ? "#fcd34d50"
                                      : "#d4b060"
                                    : D
                                      ? "#c4b5fd50"
                                      : "#b090e8",
                              background: "transparent",
                            }}
                          >
                            {entry.method}
                          </span>
                          <span
                            style={{
                              fontSize: 9,
                              fontWeight: 700,
                              padding: "2px 8px",
                              borderRadius: 4,
                              flexShrink: 0,
                              color: sc.neon,
                              background: sc.dim,
                            }}
                          >
                            {entry.sep}
                          </span>
                          <span
                            style={{
                              fontSize: 10,
                              flex: 1,
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              whiteSpace: "nowrap",
                              color: D ? "#3a5070" : "#8899bb",
                            }}
                          >
                            {entry.endpoint}
                          </span>
                          <span
                            style={{
                              fontSize: 9,
                              fontWeight: 700,
                              padding: "2px 8px",
                              borderRadius: 4,
                              border: "1px solid",
                              flexShrink: 0,
                              color: entry.success
                                ? D
                                  ? "#6ee7b7"
                                  : "#2a8a58"
                                : D
                                  ? "#ff7eb3"
                                  : "#c0005a",
                              borderColor: entry.success
                                ? D
                                  ? "#6ee7b750"
                                  : "#a7d8b8"
                                : D
                                  ? "#ff7eb350"
                                  : "#e0a0b8",
                              background: entry.success
                                ? D
                                  ? "rgba(110,231,183,0.10)"
                                  : "#f0faf5"
                                : D
                                  ? "rgba(255,126,179,0.10)"
                                  : "#fff0f5",
                            }}
                          >
                            {entry.status ?? "ERR"}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            )}
          </div>
        </main>
      </div>

      {/* Bottom HUD line */}
      <div
        style={{
          position: "fixed",
          bottom: 0,
          left: 0,
          right: 0,
          height: 1,
          background: `linear-gradient(90deg, transparent, ${neon}50, transparent)`,
          zIndex: 10,
        }}
      />

      {/* CSS keyframes */}
      <style>{`
        @import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&display=swap');
        @keyframes spin { to { transform: rotate(360deg); } }
        @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:0.4} }
        @keyframes skeleton-shimmer {
          0% { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }
        ::-webkit-scrollbar { width: 4px; height: 4px; }
        ::-webkit-scrollbar-track { background: transparent; }
        ::-webkit-scrollbar-thumb { background: ${neon}30; border-radius: 2px; }
      `}</style>
    </div>
  );
}
