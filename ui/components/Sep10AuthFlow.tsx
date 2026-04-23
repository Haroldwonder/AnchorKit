import { useState, useEffect, useRef } from "react";

// ─── Types ────────────────────────────────────────────────────────────────────
type Step =
  | "idle"
  | "connect"
  | "challenge"
  | "sign"
  | "token"
  | "authenticated";

interface WalletInfo {
  address: string;
  network: string;
}

// ─── Mock Helpers ─────────────────────────────────────────────────────────────
const mockAddress = () => {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
  let addr = "G";
  for (let i = 0; i < 55; i++)
    addr += chars[Math.floor(Math.random() * chars.length)];
  return addr;
};

const mockXDR = () => {
  const chars =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let xdr = "";
  for (let i = 0; i < 220; i++) {
    if (i > 0 && i % 64 === 0) xdr += "\n";
    xdr += chars[Math.floor(Math.random() * chars.length)];
  }
  return xdr + "==";
};

const mockJWT = () => {
  const b64 = (obj: object) => btoa(JSON.stringify(obj)).replace(/=/g, "");
  const header = b64({ alg: "HS256", typ: "JWT" });
  const payload = b64({
    iss: "testanchor.stellar.org",
    sub: mockAddress(),
    iat: Math.floor(Date.now() / 1000),
    exp: Math.floor(Date.now() / 1000) + 86400,
    jti: crypto.randomUUID(),
  });
  const sig = btoa(Math.random().toString(36).slice(2, 34)).replace(/=/g, "");
  return `${header}.${payload}.${sig}`;
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

// ─── Step Config ──────────────────────────────────────────────────────────────
const STEPS = [
  { id: "connect", label: "Connect Wallet", icon: "◎", num: 1 },
  { id: "challenge", label: "Fetch Challenge", icon: "⟁", num: 2 },
  { id: "sign", label: "Sign Challenge", icon: "✦", num: 3 },
  { id: "token", label: "Auth Token", icon: "◈", num: 4 },
];

// ─── Sub-components ───────────────────────────────────────────────────────────

function GlowRing({
  active,
  done,
  color,
}: {
  active: boolean;
  done: boolean;
  color: string;
}) {
  return (
    <div style={{ position: "relative", width: 48, height: 48, flexShrink: 0 }}>
      {/* Outer pulse */}
      {active && (
        <div
          style={{
            position: "absolute",
            inset: -6,
            borderRadius: "50%",
            border: `1px solid ${color}`,
            animation: "sep10-ping 1.4s ease-out infinite",
            opacity: 0,
          }}
        />
      )}
      {/* Ring */}
      <div
        style={{
          width: 48,
          height: 48,
          borderRadius: "50%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          border: `1.5px solid ${done ? color : active ? color : "#1e2d45"}`,
          background: done
            ? `${color}20`
            : active
              ? `${color}12`
              : "rgba(0,0,0,0.3)",
          boxShadow:
            active || done
              ? `0 0 20px ${color}40, inset 0 0 12px ${color}10`
              : "none",
          transition: "all 0.4s",
          position: "relative",
          zIndex: 1,
        }}
      >
        {done ? (
          <span
            style={{
              fontSize: 18,
              color,
              filter: `drop-shadow(0 0 6px ${color})`,
            }}
          >
            ✓
          </span>
        ) : (
          <span
            style={{
              fontSize: 16,
              color: active ? color : "#2a3d5a",
              transition: "color 0.3s",
              filter: active ? `drop-shadow(0 0 6px ${color})` : "none",
            }}
          >
            {STEPS.find((s) => s.id === (active ? "active" : ""))?.icon ?? "○"}
          </span>
        )}
      </div>
    </div>
  );
}

function Connector({ done, color }: { done: boolean; color: string }) {
  return (
    <div
      style={{
        flex: 1,
        height: 1,
        position: "relative",
        margin: "0 8px",
        overflow: "hidden",
      }}
    >
      <div style={{ position: "absolute", inset: 0, background: "#1e2d45" }} />
      <div
        style={{
          position: "absolute",
          inset: 0,
          background: color,
          boxShadow: `0 0 8px ${color}`,
          transform: done ? "scaleX(1)" : "scaleX(0)",
          transformOrigin: "left",
          transition: "transform 0.6s cubic-bezier(0.4,0,0.2,1)",
        }}
      />
    </div>
  );
}

function TokenDisplay({ jwt }: { jwt: string }) {
  const [copied, setCopied] = useState(false);
  const [revealed, setRevealed] = useState(false);
  const parts = jwt.split(".");
  const colors = ["#ff7eb3", "#79d4fd", "#7effc7"];
  const labels = ["HEADER", "PAYLOAD", "SIGNATURE"];
  const SENSITIVE_FIELDS = ["sub", "iss", "jti"];

  const copy = () => {
    navigator.clipboard.writeText(jwt);
    setCopied(true);
    setTimeout(() => setCopied(false), 1600);
  };

  const maskValue = (key: string, value: unknown): string => {
    if (!revealed && SENSITIVE_FIELDS.includes(key)) return "••••••";
    if (key === "iat" || key === "exp")
      return new Date((value as number) * 1000).toLocaleString();
    return String(value);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
      {/* Segmented JWT */}
      <div
        style={{
          fontFamily: "monospace",
          fontSize: 10,
          lineHeight: 1.7,
          padding: "14px 16px",
          borderRadius: 8,
          background: "rgba(0,0,0,0.5)",
          border: "1px solid #1e2d45",
          wordBreak: "break-all",
        }}
      >
        {parts.map((part, i) => (
          <span key={i}>
            <span
              style={{
                color: colors[i],
                textShadow: `0 0 10px ${colors[i]}60`,
              }}
            >
              {part}
            </span>
            {i < 2 && <span style={{ color: "#2a3d5a" }}>.</span>}
          </span>
        ))}
      </div>

      {/* Decoded payload */}
      {(() => {
        try {
          const payload = JSON.parse(atob(parts[1]));
          return (
            <div
              style={{
                fontSize: 10,
                fontFamily: "monospace",
                padding: "12px 14px",
                borderRadius: 8,
                background: "rgba(0,0,0,0.3)",
                border: "1px solid #1e2d45",
                display: "flex",
                flexDirection: "column",
                gap: 5,
              }}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: 4,
                }}
              >
                <div
                  style={{
                    color: "#3a5070",
                    letterSpacing: "0.15em",
                    fontSize: 9,
                  }}
                >
                  DECODED PAYLOAD
                </div>
                <button
                  onClick={() => setRevealed((r) => !r)}
                  aria-label={revealed ? "Hide sensitive JWT fields" : "Reveal sensitive JWT fields"}
                  style={{
                    padding: "3px 10px",
                    fontSize: 9,
                    fontWeight: 700,
                    letterSpacing: "0.12em",
                    textTransform: "uppercase",
                    fontFamily: "monospace",
                    borderRadius: 4,
                    cursor: "pointer",
                    border: `1px solid ${revealed ? "#ff7eb3" : "#1e2d45"}`,
                    color: revealed ? "#ff7eb3" : "#3a5070",
                    background: revealed ? "rgba(255,126,179,0.08)" : "transparent",
                  }}
                >
                  {revealed ? "HIDE" : "REVEAL"}
                </button>
              </div>
              {Object.entries(payload).map(([k, v]) => (
                <div key={k} style={{ display: "flex", gap: 10 }}>
                  <span style={{ color: "#ff7eb3", minWidth: 52 }}>{k}</span>
                  <span style={{ color: "#3a5070" }}>:</span>
                  <span
                    style={{
                      color: "#79d4fd",
                      flex: 1,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {maskValue(k, v)}
                  </span>
                </div>
              ))}
            </div>
          );
        } catch {
          return null;
        }
      })()}

      <button
        onClick={copy}
        style={{
          alignSelf: "flex-end",
          padding: "6px 14px",
          fontSize: 9,
          fontWeight: 700,
          letterSpacing: "0.15em",
          textTransform: "uppercase",
          fontFamily: "monospace",
          borderRadius: 5,
          cursor: "pointer",
          transition: "all 0.2s",
          border: `1px solid ${copied ? "#00e5ff" : "#1e2d45"}`,
          color: copied ? "#00e5ff" : "#3a5070",
          background: copied ? "rgba(0,229,255,0.08)" : "transparent",
          boxShadow: copied ? "0 0 12px rgba(0,229,255,0.2)" : "none",
        }}
      >
        {copied ? "✓ COPIED" : "COPY JWT"}
      </button>
    </div>
  );
}

function AuthStatusBadge({ wallet, jwt }: { wallet: WalletInfo; jwt: string }) {
  const [age, setAge] = useState(0);
  const [isExpired, setIsExpired] = useState(false);
  
  // Parse JWT to get expiry time
  const expiryTime = useMemo(() => {
    try {
      const parts = jwt.split(".");
      if (parts.length !== 3) return null;
      const payload = JSON.parse(atob(parts[1]));
      return payload.exp ? payload.exp * 1000 : null; // Convert to milliseconds
    } catch {
      return null;
    }
  }, [jwt]);

  // Poll token expiry every 30 seconds
  useEffect(() => {
    const checkExpiry = () => {
      if (!expiryTime) return;
      const now = Date.now();
      const expired = now >= expiryTime;
      setIsExpired(expired);
      if (!expired) {
        setAge(Math.floor((now - (expiryTime - 86400000)) / 1000));
      }
    };

    // Initial check
    checkExpiry();

    // Poll every 30 seconds
    const intervalId = setInterval(checkExpiry, 30000);

    // Also update age counter every second for display
    const ageIntervalId = setInterval(() => {
      if (!isExpired && expiryTime) {
        const now = Date.now();
        if (now >= expiryTime) {
          setIsExpired(true);
        } else {
          setAge(Math.floor((now - (expiryTime - 86400000)) / 1000));
        }
      }
    }, 1000);

    return () => {
      clearInterval(intervalId);
      clearInterval(ageIntervalId);
    };
  }, [expiryTime, isExpired]);

  const expiresIn = isExpired ? 0 : Math.max(0, 86400 - age);
  const pct = Math.max(0, (expiresIn / 86400) * 100);

  // Determine status color based on expiry state
  const statusColor = isExpired ? "#ff3670" : expiresIn < 3600 ? "#ff8c00" : "#00ff9d";
  const statusBg = isExpired ? "rgba(255,54,112,0.06)" : expiresIn < 3600 ? "rgba(255,140,0,0.06)" : "rgba(0,255,157,0.06)";
  const statusBorder = isExpired ? "rgba(255,54,112,0.3)" : expiresIn < 3600 ? "rgba(255,140,0,0.3)" : "rgba(0,255,157,0.3)";
  const statusLabel = isExpired ? "EXPIRED" : "AUTHENTICATED";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {/* Status banner */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          padding: "14px 16px",
          borderRadius: 10,
          border: `1px solid ${statusBorder}`,
          background: statusBg,
          boxShadow: `0 0 30px ${statusColor}08`,
        }}
      >
        <div
          style={{
            width: 10,
            height: 10,
            borderRadius: "50%",
            background: statusColor,
            boxShadow: `0 0 14px ${statusColor}`,
            flexShrink: 0,
            animation: isExpired ? "none" : "sep10-pulse 2s infinite",
          }}
        />
        <div style={{ flex: 1 }}>
          <div
            style={{
              fontSize: 12,
              fontWeight: 700,
              color: statusColor,
              letterSpacing: "0.1em",
            }}
          >
            {statusLabel}
          </div>
          <div style={{ fontSize: 10, color: "#3a5070", marginTop: 2 }}>
            {isExpired ? "Token has expired · Re-authenticate required" : "Session active · SEP-10 verified"}
          </div>
        </div>
        <div style={{ textAlign: "right" }}>
          <div
            style={{ fontSize: 9, color: "#3a5070", letterSpacing: "0.1em" }}
          >
            {isExpired ? "EXPIRED" : "EXPIRES IN"}
          </div>
          <div
            style={{
              fontSize: 13,
              fontWeight: 700,
              color: statusColor,
              fontFamily: "monospace",
            }}
          >
            {isExpired ? "0h 0m 0s" : `${Math.floor(expiresIn / 3600)}h ${Math.floor((expiresIn % 3600) / 60)}m ${expiresIn % 60}s`}
          </div>
        </div>
      </div>

      {/* Token expiry bar */}
      <div>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            fontSize: 9,
            color: "#3a5070",
            marginBottom: 5,
          }}
        >
          <span>TOKEN VALIDITY</span>
          <span>{pct.toFixed(1)}%</span>
        </div>
        <div
          style={{
            height: 4,
            borderRadius: 2,
            background: "#0d1628",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              height: "100%",
              borderRadius: 2,
              width: `${pct}%`,
              background: isExpired 
                ? "linear-gradient(90deg,#ff3670,#ff3670)" 
                : expiresIn < 3600 
                  ? "linear-gradient(90deg,#ff8c00,#ff8c00)"
                  : "linear-gradient(90deg,#00e5ff,#00ff9d)",
              boxShadow: `0 0 8px ${statusColor}60`,
              transition: "width 1s linear, background 0.3s",
            }}
          />
        </div>
      </div>

      {/* Wallet info grid */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
        {[
          {
            label: "ACCOUNT",
            value: `${wallet.address.slice(0, 6)}...${wallet.address.slice(-6)}`,
          },
          { label: "NETWORK", value: wallet.network },
          { label: "PROTOCOL", value: "SEP-10" },
          { label: "AUTH METHOD", value: "ED25519" },
        ].map(({ label, value }) => (
          <div
            key={label}
            style={{
              padding: "10px 12px",
              borderRadius: 7,
              border: "1px solid #131f32",
              background: "rgba(0,0,0,0.25)",
            }}
          >
            <div
              style={{
                fontSize: 8,
                color: "#2a3d5a",
                letterSpacing: "0.15em",
                marginBottom: 4,
              }}
            >
              {label}
            </div>
            <div
              style={{
                fontSize: 11,
                color: "#8aaad4",
                fontFamily: "monospace",
              }}
            >
              {value}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ─── Main ─────────────────────────────────────────────────────────────────────
export default function SEP10AuthFlow() {
  const [step, setStep] = useState<Step>("idle");
  const [wallet, setWallet] = useState<WalletInfo | null>(null);
  const [challenge, setChallenge] = useState<string | null>(null);
  const [signedXdr, setSignedXdr] = useState<string | null>(null);
  const [jwt, setJwt] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [domain, setDomain] = useState("testanchor.stellar.org");
  const [log, setLog] = useState<string[]>([]);
  const logRef = useRef<HTMLDivElement>(null);

  const addLog = (msg: string) =>
    setLog((prev) => [
      ...prev.slice(-40),
      `[${new Date().toLocaleTimeString()}] ${msg}`,
    ]);

  useEffect(() => {
    if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight;
  }, [log]);

  const NEON = "#00e5ff";

  const completedSteps = (() => {
    const map: Record<string, boolean> = {};
    if (wallet) map["connect"] = true;
    if (challenge) map["challenge"] = true;
    if (signedXdr) map["sign"] = true;
    if (jwt) map["token"] = true;
    return map;
  })();

  const activeStep = jwt
    ? "token"
    : signedXdr
      ? "sign"
      : challenge
        ? "challenge"
        : wallet
          ? "connect"
          : "";

  // ── Step handlers ──
  const connectWallet = async () => {
    setLoading(true);
    setError(null);
    addLog("Requesting wallet connection...");
    await sleep(900);
    addLog("Scanning for available wallets...");
    await sleep(700);
    const addr = mockAddress();
    addLog(`Wallet found: ${addr.slice(0, 8)}...`);
    await sleep(400);
    addLog("Connection approved ✓");
    setWallet({ address: addr, network: "Testnet" });
    setStep("challenge");
    setLoading(false);
  };

  const fetchChallenge = async () => {
    setLoading(true);
    setError(null);
    addLog(
      `GET https://${domain}/auth?account=${wallet?.address.slice(0, 8)}...`,
    );
    await sleep(600);
    addLog("Response: 200 OK");
    await sleep(300);
    const xdr = mockXDR();
    addLog(`Challenge XDR received (${xdr.length} chars)`);
    setChallenge(xdr);
    setStep("sign");
    setLoading(false);
  };

  const signChallenge = async () => {
    setLoading(true);
    setError(null);
    addLog("Sending challenge XDR to wallet for signing...");
    await sleep(800);
    addLog("User approved signature request");
    await sleep(600);
    const signed = mockXDR();
    addLog("Transaction signed with ED25519 key ✓");
    setSignedXdr(signed);
    setStep("token");
    setLoading(false);
  };

  const submitChallenge = async () => {
    setLoading(true);
    setError(null);
    addLog(`POST https://${domain}/auth`);
    addLog("Sending signed XDR...");
    await sleep(700);
    addLog("Response: 200 OK");
    await sleep(300);
    const token = mockJWT();
    addLog("JWT received ✓");
    addLog("Auth session established — expires in 24h");
    setJwt(token);
    setStep("authenticated");
    setLoading(false);
  };

  const reset = () => {
    setStep("idle");
    setWallet(null);
    setChallenge(null);
    setSignedXdr(null);
    setJwt(null);
    setError(null);
    addLog("─── Session reset ───");
  };

  // ─ Step cards config ─
  const stepCards = [
    {
      id: "connect",
      title: "Connect Wallet",
      icon: "◎",
      subtitle: "Link your Stellar keypair",
      ready: step === "idle" || step === "connect",
      done: !!wallet,
      content: wallet ? (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 10,
              padding: "12px 14px",
              borderRadius: 8,
              background: "rgba(0,229,255,0.06)",
              border: "1px solid rgba(0,229,255,0.2)",
            }}
          >
            <div
              style={{
                width: 36,
                height: 36,
                borderRadius: "50%",
                background: "linear-gradient(135deg,#00e5ff22,#00ff9d22)",
                border: "1px solid #00e5ff40",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: 18,
                flexShrink: 0,
              }}
            >
              ◉
            </div>
            <div>
              <div
                style={{
                  fontSize: 10,
                  color: "#3a5070",
                  letterSpacing: "0.12em",
                  marginBottom: 3,
                }}
              >
                CONNECTED ACCOUNT
              </div>
              <div
                style={{
                  fontSize: 11,
                  fontFamily: "monospace",
                  color: "#8aaad4",
                }}
              >
                {wallet.address.slice(0, 12)}...{wallet.address.slice(-8)}
              </div>
            </div>
            <div
              style={{
                marginLeft: "auto",
                fontSize: 9,
                padding: "3px 8px",
                borderRadius: 4,
                background: "rgba(0,255,157,0.1)",
                color: "#00ff9d",
                border: "1px solid rgba(0,255,157,0.3)",
              }}
            >
              {wallet.network}
            </div>
          </div>
        </div>
      ) : (
        <div>
          <p
            style={{
              fontSize: 11,
              color: "#3a5070",
              lineHeight: 1.6,
              marginBottom: 14,
            }}
          >
            Connect your Stellar wallet to begin the SEP-10 authentication
            handshake. Your private key never leaves your wallet.
          </p>
          <button
            onClick={connectWallet}
            disabled={loading}
            style={btnStyle(NEON, loading)}
          >
            {loading ? <Spinner /> : <>{walletIcon} Connect Wallet</>}
          </button>
        </div>
      ),
    },
    {
      id: "challenge",
      title: "Fetch Challenge",
      icon: "⟁",
      subtitle: "Server issues a signed transaction",
      ready: !!wallet,
      done: !!challenge,
      content: challenge ? (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div
            style={{
              fontSize: 9,
              color: "#3a5070",
              letterSpacing: "0.12em",
              marginBottom: 4,
            }}
          >
            CHALLENGE XDR
          </div>
          <div
            style={{
              fontFamily: "monospace",
              fontSize: 10,
              lineHeight: 1.6,
              padding: "12px 14px",
              borderRadius: 8,
              background: "rgba(0,0,0,0.4)",
              border: "1px solid #131f32",
              wordBreak: "break-all",
              color: "#79d4fd",
              maxHeight: 90,
              overflowY: "auto",
            }}
          >
            {challenge.slice(0, 160)}…
          </div>
          <div style={{ fontSize: 9, color: "#2a3d5a" }}>
            Received from{" "}
            <span style={{ color: "#3a5070" }}>{domain}/auth</span>
          </div>
        </div>
      ) : (
        <div>
          <p
            style={{
              fontSize: 11,
              color: "#3a5070",
              lineHeight: 1.6,
              marginBottom: 14,
            }}
          >
            The anchor generates a unique challenge transaction. This XDR must
            be signed by your wallet to prove key ownership.
          </p>
          <button
            onClick={fetchChallenge}
            disabled={loading || !wallet}
            style={btnStyle(NEON, loading || !wallet)}
          >
            {loading ? <Spinner /> : <>{fetchIcon} Fetch Challenge</>}
          </button>
        </div>
      ),
    },
    {
      id: "sign",
      title: "Sign Challenge",
      icon: "✦",
      subtitle: "Prove keypair ownership",
      ready: !!challenge,
      done: !!signedXdr,
      content: signedXdr ? (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div
            style={{
              fontSize: 9,
              color: "#3a5070",
              letterSpacing: "0.12em",
              marginBottom: 4,
            }}
          >
            SIGNED XDR
          </div>
          <div
            style={{
              fontFamily: "monospace",
              fontSize: 10,
              lineHeight: 1.6,
              padding: "12px 14px",
              borderRadius: 8,
              background: "rgba(0,0,0,0.4)",
              border: "1px solid #131f32",
              wordBreak: "break-all",
              color: "#7effc7",
              maxHeight: 90,
              overflowY: "auto",
            }}
          >
            {signedXdr.slice(0, 160)}…
          </div>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              fontSize: 9,
              color: "#00ff9d",
            }}
          >
            <span>✓</span> ED25519 signature applied
          </div>
        </div>
      ) : (
        <div>
          <p
            style={{
              fontSize: 11,
              color: "#3a5070",
              lineHeight: 1.6,
              marginBottom: 14,
            }}
          >
            Your wallet signs the challenge XDR with your private key. This
            creates cryptographic proof that you control the account — no
            password needed.
          </p>
          <button
            onClick={signChallenge}
            disabled={loading || !challenge}
            style={btnStyle("#7effc7", loading || !challenge)}
          >
            {loading ? <Spinner /> : <>{signIcon} Sign with Wallet</>}
          </button>
        </div>
      ),
    },
    {
      id: "token",
      title: "Auth Token",
      icon: "◈",
      subtitle: "Receive your JWT bearer token",
      ready: !!signedXdr,
      done: !!jwt,
      content: jwt ? (
        <TokenDisplay jwt={jwt} />
      ) : (
        <div>
          <p
            style={{
              fontSize: 11,
              color: "#3a5070",
              lineHeight: 1.6,
              marginBottom: 14,
            }}
          >
            Submit the signed XDR back to the anchor. If valid, a JWT is issued
            — use it as a{" "}
            <code style={{ color: "#79d4fd", fontSize: 10 }}>Bearer</code> token
            in all subsequent SEP requests.
          </p>
          <button
            onClick={submitChallenge}
            disabled={loading || !signedXdr}
            style={btnStyle("#ff7eb3", loading || !signedXdr)}
          >
            {loading ? <Spinner /> : <>{submitIcon} Submit & Get Token</>}
          </button>
        </div>
      ),
    },
  ];

  return (
    <div
      style={{
        fontFamily: "'JetBrains Mono','Fira Code',monospace",
        minHeight: "100vh",
        background: "#050810",
        color: "#c8d8ee",
        position: "relative",
        overflow: "hidden",
      }}
    >
      <style>{`
        @import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&display=swap');
        @keyframes sep10-ping { 0%{transform:scale(1);opacity:0.7} 100%{transform:scale(1.9);opacity:0} }
        @keyframes sep10-pulse { 0%,100%{opacity:1} 50%{opacity:0.4} }
        @keyframes sep10-spin { to{transform:rotate(360deg)} }
        @keyframes sep10-slide-in { from{opacity:0;transform:translateY(10px)} to{opacity:1;transform:translateY(0)} }
        @keyframes sep10-float { 0%,100%{transform:translateY(0)} 50%{transform:translateY(-8px)} }
        ::-webkit-scrollbar{width:4px;height:4px}
        ::-webkit-scrollbar-track{background:transparent}
        ::-webkit-scrollbar-thumb{background:#1e2d4580;border-radius:2px}
      `}</style>

      {/* Grid */}
      <div
        style={{
          position: "fixed",
          inset: 0,
          pointerEvents: "none",
          backgroundImage:
            "linear-gradient(rgba(0,229,255,0.03) 1px,transparent 1px),linear-gradient(90deg,rgba(0,229,255,0.03) 1px,transparent 1px)",
          backgroundSize: "44px 44px",
        }}
      />

      {/* Glows */}
      <div
        style={{
          position: "fixed",
          top: -200,
          left: "10%",
          width: 600,
          height: 600,
          borderRadius: "50%",
          background: "#00e5ff",
          opacity: 0.04,
          filter: "blur(120px)",
          pointerEvents: "none",
        }}
      />
      <div
        style={{
          position: "fixed",
          bottom: -100,
          right: "5%",
          width: 400,
          height: 400,
          borderRadius: "50%",
          background: "#00ff9d",
          opacity: 0.03,
          filter: "blur(100px)",
          pointerEvents: "none",
        }}
      />

      {/* Corner brackets */}
      {["tl", "tr", "bl", "br"].map((pos) => (
        <div
          key={pos}
          style={{
            position: "fixed",
            width: 24,
            height: 24,
            opacity: 0.3,
            pointerEvents: "none",
            top: pos.includes("t") ? 10 : undefined,
            bottom: pos.includes("b") ? 10 : undefined,
            left: pos.includes("l") ? 10 : undefined,
            right: pos.includes("r") ? 10 : undefined,
            borderTop: pos.includes("t") ? `1.5px solid ${NEON}` : undefined,
            borderBottom: pos.includes("b") ? `1.5px solid ${NEON}` : undefined,
            borderLeft: pos.includes("l") ? `1.5px solid ${NEON}` : undefined,
            borderRight: pos.includes("r") ? `1.5px solid ${NEON}` : undefined,
          }}
        />
      ))}

      <div
        style={{
          maxWidth: 960,
          margin: "0 auto",
          padding: "32px 24px",
          position: "relative",
          zIndex: 1,
        }}
      >
        {/* ── Header ── */}
        <div
          style={{
            display: "flex",
            alignItems: "flex-start",
            justifyContent: "space-between",
            marginBottom: 36,
          }}
        >
          <div>
            <div
              style={{
                fontSize: 10,
                letterSpacing: "0.25em",
                color: "#3a5070",
                textTransform: "uppercase",
                marginBottom: 8,
              }}
            >
              ◈ Stellar · SEP-10
            </div>
            <h1
              style={{
                fontSize: 26,
                fontWeight: 700,
                letterSpacing: "-0.02em",
                color: "#dde6f5",
                margin: 0,
                lineHeight: 1.2,
              }}
            >
              Authentication
              <br />
              <span style={{ color: NEON, textShadow: `0 0 30px ${NEON}60` }}>
                Flow
              </span>
            </h1>
            <p
              style={{
                marginTop: 10,
                fontSize: 11,
                color: "#3a5070",
                lineHeight: 1.6,
                maxWidth: 340,
              }}
            >
              Challenge-response authentication using your Stellar keypair. No
              passwords. Cryptographic proof of account ownership.
            </p>
          </div>

          {/* Domain + reset */}
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: 8,
              alignItems: "flex-end",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                padding: "7px 12px",
                borderRadius: 7,
                border: "1px solid #131f32",
                background: "rgba(0,0,0,0.35)",
              }}
            >
              <span style={{ fontSize: 10, color: "#2a3d5a" }}>https://</span>
              <input
                value={domain}
                onChange={(e) => setDomain(e.target.value)}
                style={{
                  background: "transparent",
                  border: "none",
                  outline: "none",
                  color: NEON,
                  fontSize: 10,
                  fontFamily: "inherit",
                  width: 200,
                }}
                placeholder="anchor.example.com"
              />
            </div>
            {step !== "idle" && (
              <button
                onClick={reset}
                style={{
                  fontSize: 9,
                  fontWeight: 700,
                  letterSpacing: "0.15em",
                  textTransform: "uppercase",
                  cursor: "pointer",
                  fontFamily: "inherit",
                  padding: "5px 12px",
                  borderRadius: 5,
                  border: "1px solid #1e2d45",
                  background: "transparent",
                  color: "#2a3d5a",
                  transition: "all 0.2s",
                }}
              >
                ↺ RESET
              </button>
            )}
          </div>
        </div>

        {/* ── Progress bar ── */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            marginBottom: 40,
            padding: "0 4px",
          }}
        >
          {STEPS.map((s, i) => {
            const isDone = completedSteps[s.id];
            const isActive =
              (!isDone && activeStep === s.id) ||
              (!activeStep && i === 0 && step === "idle");
            const stepColor =
              s.id === "sign" ? "#7effc7" : s.id === "token" ? "#ff7eb3" : NEON;
            return (
              <div
                key={s.id}
                style={{ display: "flex", alignItems: "center", flex: 1 }}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 10,
                    flexShrink: 0,
                  }}
                >
                  <GlowRing active={isActive} done={isDone} color={stepColor} />
                  <div style={{ display: i < 3 ? "none" : undefined }}>
                    {/* hide labels for middle steps on small screens */}
                  </div>
                </div>
                {i < STEPS.length - 1 && (
                  <Connector
                    done={
                      completedSteps[STEPS[i + 1]?.id] || completedSteps[s.id]
                    }
                    color={stepColor}
                  />
                )}
              </div>
            );
          })}
        </div>

        {/* Step labels row */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(4, 1fr)",
            marginBottom: 32,
            gap: 4,
          }}
        >
          {STEPS.map((s) => {
            const isDone = completedSteps[s.id];
            const stepColor =
              s.id === "sign" ? "#7effc7" : s.id === "token" ? "#ff7eb3" : NEON;
            return (
              <div key={s.id} style={{ textAlign: "center" }}>
                <div
                  style={{
                    fontSize: 11,
                    fontWeight: 700,
                    color: isDone ? stepColor : "#2a3d5a",
                    letterSpacing: "0.05em",
                    transition: "color 0.3s",
                  }}
                >
                  {s.label}
                </div>
                <div
                  style={{
                    fontSize: 9,
                    color: "#1e2d45",
                    marginTop: 2,
                    letterSpacing: "0.1em",
                  }}
                >
                  STEP {s.num}
                </div>
              </div>
            );
          })}
        </div>

        {/* ── Step Cards Grid ── */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(2, 1fr)",
            gap: 16,
            marginBottom: 24,
          }}
        >
          {stepCards.map((card) => {
            const isActive = !card.done && card.ready && !jwt;
            const stepColor =
              card.id === "sign"
                ? "#7effc7"
                : card.id === "token"
                  ? "#ff7eb3"
                  : NEON;
            return (
              <div
                key={card.id}
                style={{
                  borderRadius: 12,
                  padding: "20px 20px",
                  border: `1px solid ${card.done ? `${stepColor}35` : isActive ? `${stepColor}25` : "#0f1a28"}`,
                  background: card.done
                    ? `${stepColor}06`
                    : isActive
                      ? "rgba(0,0,0,0.45)"
                      : "rgba(0,0,0,0.2)",
                  boxShadow: card.done
                    ? `0 0 40px ${stepColor}08`
                    : isActive
                      ? `0 0 30px ${stepColor}05`
                      : "none",
                  transition: "all 0.4s",
                  animation: isActive ? "sep10-slide-in 0.35s ease" : undefined,
                  opacity: !card.ready && !card.done ? 0.35 : 1,
                  position: "relative",
                  overflow: "hidden",
                }}
              >
                {/* Active shimmer */}
                {isActive && (
                  <div
                    style={{
                      position: "absolute",
                      top: 0,
                      left: "-100%",
                      width: "60%",
                      height: "100%",
                      background: `linear-gradient(90deg, transparent, ${stepColor}06, transparent)`,
                      animation: "sep10-float 3s ease-in-out infinite",
                      pointerEvents: "none",
                    }}
                  />
                )}

                {/* Card header */}
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 10,
                    marginBottom: 14,
                  }}
                >
                  <div
                    style={{
                      width: 32,
                      height: 32,
                      borderRadius: 8,
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      fontSize: 15,
                      border: `1px solid ${card.done ? `${stepColor}50` : isActive ? `${stepColor}35` : "#131f32"}`,
                      background: card.done
                        ? `${stepColor}15`
                        : isActive
                          ? `${stepColor}08`
                          : "transparent",
                      color: card.done
                        ? stepColor
                        : isActive
                          ? stepColor
                          : "#1e2d45",
                      boxShadow:
                        card.done || isActive
                          ? `0 0 14px ${stepColor}30`
                          : "none",
                      transition: "all 0.4s",
                    }}
                  >
                    {card.done ? "✓" : card.icon}
                  </div>
                  <div>
                    <div
                      style={{
                        fontSize: 12,
                        fontWeight: 700,
                        color: card.done
                          ? stepColor
                          : isActive
                            ? "#c8d8ee"
                            : "#2a3d5a",
                        letterSpacing: "0.04em",
                        transition: "color 0.3s",
                      }}
                    >
                      {card.title}
                    </div>
                    <div
                      style={{
                        fontSize: 9,
                        color: "#2a3d5a",
                        marginTop: 1,
                        letterSpacing: "0.1em",
                      }}
                    >
                      {card.subtitle}
                    </div>
                  </div>
                  {card.done && (
                    <div
                      style={{
                        marginLeft: "auto",
                        fontSize: 9,
                        padding: "2px 8px",
                        borderRadius: 10,
                        background: `${stepColor}15`,
                        color: stepColor,
                        border: `1px solid ${stepColor}30`,
                      }}
                    >
                      COMPLETE
                    </div>
                  )}
                </div>

                {/* Card content */}
                <div>{card.content}</div>
              </div>
            );
          })}
        </div>

        {/* ── Auth Status (full width, post-auth) ── */}
        {step === "authenticated" && wallet && (
          <div
            style={{
              borderRadius: 12,
              padding: "24px 24px",
              border: "1px solid rgba(0,255,157,0.2)",
              background: "rgba(0,255,157,0.04)",
              boxShadow: "0 0 60px rgba(0,255,157,0.06)",
              animation: "sep10-slide-in 0.4s ease",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 10,
                marginBottom: 18,
              }}
            >
              <div
                style={{
                  width: 32,
                  height: 32,
                  borderRadius: 8,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: 16,
                  border: "1px solid rgba(0,255,157,0.4)",
                  background: "rgba(0,255,157,0.1)",
                  color: "#00ff9d",
                  boxShadow: "0 0 16px rgba(0,255,157,0.25)",
                }}
              >
                ◉
              </div>
              <div>
                <div
                  style={{
                    fontSize: 12,
                    fontWeight: 700,
                    color: "#00ff9d",
                    letterSpacing: "0.06em",
                  }}
                >
                  Auth Status
                </div>
                <div
                  style={{
                    fontSize: 9,
                    color: "#2a3d5a",
                    marginTop: 1,
                    letterSpacing: "0.1em",
                  }}
                >
                  Live session monitor
                </div>
              </div>
            </div>
            <AuthStatusBadge wallet={wallet} jwt={jwt} />
          </div>
        )}

        {/* ── Activity Log ── */}
        {log.length > 0 && (
          <div
            style={{
              marginTop: 20,
              borderRadius: 10,
              overflow: "hidden",
              border: "1px solid #0f1a28",
            }}
          >
            <div
              style={{
                padding: "8px 14px",
                background: "rgba(0,0,0,0.6)",
                borderBottom: "1px solid #0f1a28",
                display: "flex",
                alignItems: "center",
                gap: 8,
              }}
            >
              <div
                style={{
                  width: 7,
                  height: 7,
                  borderRadius: "50%",
                  background: NEON,
                  animation: "sep10-pulse 2s infinite",
                  boxShadow: `0 0 8px ${NEON}`,
                }}
              />
              <span
                style={{
                  fontSize: 9,
                  fontWeight: 700,
                  letterSpacing: "0.2em",
                  color: "#2a3d5a",
                  textTransform: "uppercase",
                }}
              >
                Activity Log
              </span>
            </div>
            <div
              ref={logRef}
              style={{
                padding: "12px 14px",
                background: "rgba(0,0,0,0.45)",
                maxHeight: 130,
                overflowY: "auto",
                display: "flex",
                flexDirection: "column",
                gap: 3,
              }}
            >
              {log.map((entry, i) => (
                <div
                  key={i}
                  style={{
                    fontSize: 10,
                    fontFamily: "monospace",
                    color: i === log.length - 1 ? "#8aaad4" : "#2a3d5a",
                    transition: "color 0.3s",
                  }}
                >
                  {entry}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Bottom HUD */}
        <div
          style={{
            marginTop: 32,
            display: "flex",
            justifyContent: "center",
            gap: 6,
          }}
        >
          {STEPS.map((s) => (
            <div
              key={s.id}
              style={{
                width: completedSteps[s.id] ? 20 : 6,
                height: 3,
                borderRadius: 2,
                background: completedSteps[s.id] ? NEON : "#131f32",
                boxShadow: completedSteps[s.id] ? `0 0 8px ${NEON}` : "none",
                transition: "all 0.4s",
              }}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

// ─── Inline helpers ────────────────────────────────────────────────────────────
function btnStyle(color: string, disabled: boolean): React.CSSProperties {
  return {
    display: "inline-flex",
    alignItems: "center",
    gap: 8,
    padding: "10px 18px",
    borderRadius: 7,
    fontSize: 10,
    fontWeight: 700,
    letterSpacing: "0.15em",
    textTransform: "uppercase",
    fontFamily: "inherit",
    cursor: disabled ? "not-allowed" : "pointer",
    transition: "all 0.2s",
    border: `1px solid ${disabled ? "#131f32" : color}`,
    background: disabled ? "transparent" : `${color}14`,
    color: disabled ? "#1e2d45" : color,
    boxShadow: disabled ? "none" : `0 0 18px ${color}28`,
  };
}

const Spinner = () => (
  <svg
    style={{
      width: 13,
      height: 13,
      animation: "sep10-spin 0.7s linear infinite",
    }}
    fill="none"
    viewBox="0 0 24 24"
  >
    <circle
      style={{ opacity: 0.2 }}
      cx="12"
      cy="12"
      r="10"
      stroke="currentColor"
      strokeWidth="3"
    />
    <path
      style={{ opacity: 0.9 }}
      fill="currentColor"
      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
    />
  </svg>
);

const walletIcon = (
  <svg
    style={{ width: 13, height: 13 }}
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M3 10h18M7 15h1m4 0h1m-7 4h12a3 3 0 003-3V8a3 3 0 00-3-3H6a3 3 0 00-3 3v8a3 3 0 003 3z"
    />
  </svg>
);
const fetchIcon = (
  <svg
    style={{ width: 13, height: 13 }}
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
    />
  </svg>
);
const signIcon = (
  <svg
    style={{ width: 13, height: 13 }}
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"
    />
  </svg>
);
const submitIcon = (
  <svg
    style={{ width: 13, height: 13 }}
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
    />
  </svg>
);
