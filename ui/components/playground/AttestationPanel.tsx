import React, { useState } from "react";

export interface AttestationEntry {
  /** Unique identifier for the attestation record */
  id: string;
  /** Stellar address of the subject whose attestation is recorded */
  subject: string;
  /** Hex-encoded SHA-256 payload hash that was attested on-chain */
  payloadHash: string;
  /** Unix timestamp (seconds) recorded in the attestation */
  timestamp: number;
  /** Whether this attestation is still considered valid */
  valid: boolean;
}

export interface AttestationPanelProps {
  /** List of attestation records to display */
  attestations: AttestationEntry[];
  /** Called when the user requests to submit a new attestation */
  onSubmit?: (subject: string, payloadHash: string) => void;
  /** Optional CSS class applied to the root element */
  className?: string;
}

function formatTimestamp(ts: number): string {
  return new Date(ts * 1000).toLocaleString();
}

function truncate(str: string, max = 16): string {
  if (str.length <= max) return str;
  return `${str.slice(0, 8)}…${str.slice(-8)}`;
}

/**
 * AttestationPanel
 *
 * Displays a list of on-chain attestation records and provides a simple form
 * to submit a new attestation (subject address + payload hash).
 */
export function AttestationPanel({
  attestations,
  onSubmit,
  className,
}: AttestationPanelProps) {
  const [subject, setSubject] = useState("");
  const [payloadHash, setPayloadHash] = useState("");
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError(null);

    if (!subject.trim()) {
      setError("Subject address is required.");
      return;
    }
    if (!/^[0-9a-fA-F]{64}$/.test(payloadHash.trim())) {
      setError("Payload hash must be a 64-character hex string.");
      return;
    }

    onSubmit?.(subject.trim(), payloadHash.trim().toLowerCase());
    setSubject("");
    setPayloadHash("");
  };

  return (
    <section
      className={className}
      aria-label="Attestation panel"
      style={{ display: "flex", flexDirection: "column", gap: 24 }}
    >
      {/* ── Submit form ─────────────────────────────────────────────────── */}
      {onSubmit && (
        <form
          onSubmit={handleSubmit}
          aria-label="Submit attestation"
          style={{ display: "flex", flexDirection: "column", gap: 12 }}
        >
          <h2 style={{ margin: 0, fontSize: 16, fontWeight: 700 }}>
            Submit Attestation
          </h2>

          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label htmlFor="attestation-subject" style={{ fontSize: 13, fontWeight: 600 }}>
              Subject address
            </label>
            <input
              id="attestation-subject"
              type="text"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder="G…"
              autoComplete="off"
              spellCheck={false}
              style={{
                fontFamily: "monospace",
                fontSize: 13,
                padding: "6px 10px",
                borderRadius: 6,
                border: "1px solid var(--ak-border, #d1d5db)",
                background: "var(--ak-surface, #fff)",
                color: "var(--ak-text, #111)",
                width: "100%",
                boxSizing: "border-box",
              }}
            />
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label htmlFor="attestation-payload-hash" style={{ fontSize: 13, fontWeight: 600 }}>
              Payload hash (64-char hex)
            </label>
            <input
              id="attestation-payload-hash"
              type="text"
              value={payloadHash}
              onChange={(e) => setPayloadHash(e.target.value)}
              placeholder="0000000000000000000000000000000000000000000000000000000000000000"
              autoComplete="off"
              spellCheck={false}
              maxLength={64}
              style={{
                fontFamily: "monospace",
                fontSize: 13,
                padding: "6px 10px",
                borderRadius: 6,
                border: "1px solid var(--ak-border, #d1d5db)",
                background: "var(--ak-surface, #fff)",
                color: "var(--ak-text, #111)",
                width: "100%",
                boxSizing: "border-box",
              }}
            />
          </div>

          {error && (
            <p
              role="alert"
              style={{ margin: 0, fontSize: 13, color: "#dc2626" }}
            >
              {error}
            </p>
          )}

          <button
            type="submit"
            style={{
              alignSelf: "flex-start",
              padding: "8px 18px",
              borderRadius: 8,
              border: "none",
              background: "#2563eb",
              color: "#fff",
              fontWeight: 600,
              fontSize: 14,
              cursor: "pointer",
            }}
          >
            Submit
          </button>
        </form>
      )}

      {/* ── Attestation list ────────────────────────────────────────────── */}
      <div>
        <h2 style={{ margin: "0 0 12px", fontSize: 16, fontWeight: 700 }}>
          Attestations
          {attestations.length > 0 && (
            <span
              style={{
                marginLeft: 8,
                fontSize: 12,
                fontWeight: 600,
                padding: "1px 8px",
                borderRadius: 12,
                background: "var(--ak-surface, #f3f4f6)",
                color: "var(--ak-text-muted, #6b7280)",
              }}
            >
              {attestations.length}
            </span>
          )}
        </h2>

        {attestations.length === 0 ? (
          <p
            role="status"
            style={{ margin: 0, fontSize: 13, color: "var(--ak-text-muted, #6b7280)" }}
          >
            No attestations recorded yet.
          </p>
        ) : (
          <ul
            role="list"
            aria-label="Attestation records"
            style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 8 }}
          >
            {attestations.map((entry) => (
              <li
                key={entry.id}
                role="listitem"
                aria-label={`Attestation ${entry.id}`}
                style={{
                  padding: "10px 14px",
                  borderRadius: 8,
                  border: "1px solid var(--ak-border, #e5e7eb)",
                  background: "var(--ak-surface, #fff)",
                  display: "flex",
                  flexDirection: "column",
                  gap: 4,
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span style={{ fontFamily: "monospace", fontSize: 12, color: "var(--ak-text-muted, #6b7280)" }}>
                    ID: {entry.id}
                  </span>
                  <span
                    aria-label={entry.valid ? "Valid" : "Revoked"}
                    style={{
                      fontSize: 11,
                      fontWeight: 700,
                      padding: "1px 8px",
                      borderRadius: 12,
                      background: entry.valid ? "#dcfce7" : "#fee2e2",
                      color: entry.valid ? "#16a34a" : "#dc2626",
                      textTransform: "uppercase",
                      letterSpacing: "0.06em",
                    }}
                  >
                    {entry.valid ? "Valid" : "Revoked"}
                  </span>
                </div>

                <div style={{ fontSize: 13, color: "var(--ak-text, #111)" }}>
                  <span style={{ fontWeight: 600 }}>Subject: </span>
                  <span style={{ fontFamily: "monospace" }}>
                    {truncate(entry.subject, 24)}
                  </span>
                </div>

                <div style={{ fontSize: 13, color: "var(--ak-text, #111)" }}>
                  <span style={{ fontWeight: 600 }}>Hash: </span>
                  <span style={{ fontFamily: "monospace" }}>
                    {truncate(entry.payloadHash, 24)}
                  </span>
                </div>

                <div style={{ fontSize: 12, color: "var(--ak-text-muted, #6b7280)" }}>
                  {formatTimestamp(entry.timestamp)}
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

export default AttestationPanel;
