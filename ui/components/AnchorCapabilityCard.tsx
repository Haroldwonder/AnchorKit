import { useState } from "react";
import { useTheme } from "../hooks/useTheme";

// ─── Types ────────────────────────────────────────────────────────────────────

export type KYCLevel = "none" | "basic" | "full" | "enhanced";
export type OperationType = "deposit" | "withdrawal" | "both";

export interface AssetFee {
  type: "flat" | "percent" | "tiered";
  flatAmount?: number;
  percent?: number;
  tiers?: Array<{ upTo: number | null; fee: string }>;
  currency: string;
}

export interface AssetLimits {
  minDeposit?: number;
  maxDeposit?: number;
  minWithdrawal?: number;
  maxWithdrawal?: number;
  dailyLimit?: number;
  monthlyLimit?: number;
  currency: string;
}

export interface KYCField {
  name: string;
  label: string;
  required: boolean;
}

export interface KYCRequirement {
  level: KYCLevel;
  fields: KYCField[];
  documentTypes?: string[]; // e.g. ["passport", "drivers_license", "utility_bill"]
  estimatedTime?: string;
  description?: string;
}

export interface SupportedAsset {
  code: string;
  issuer?: string;
  name: string;
  icon?: string; // emoji or letter avatar fallback
  operationTypes: OperationType[];
  depositEnabled: boolean;
  withdrawalEnabled: boolean;
  fees: { deposit?: AssetFee; withdrawal?: AssetFee };
  limits: AssetLimits;
  kyc: KYCRequirement;
  countries?: string[]; // ISO-3166 alpha-2
  networks?: string[]; // e.g. "ACH", "SEPA", "SWIFT"
}

/** Per-service health status for the capability card */
export type ServiceHealth = "healthy" | "degraded" | "unavailable";

/** Which on-chain services the anchor exposes */
export interface AnchorServices {
  deposits?: boolean;
  withdrawals?: boolean;
  quotes?: boolean;
  kyc?: boolean;
}

/** Optional health status per service */
export interface AnchorHealthStatus {
  deposits?: ServiceHealth;
  withdrawals?: ServiceHealth;
  quotes?: ServiceHealth;
  kyc?: ServiceHealth;
}

export interface AnchorCapabilityCardProps {
  anchorName: string;
  domain: string;
  logoInitials?: string;
  accentColor?: string;
  description?: string;
  assets: SupportedAsset[];
  /** Shows a skeleton loader in the tab content area */
  isLoading?: boolean;
  /** Renders an error state in the tab content area */
  error?: string;
  /** Which services this anchor supports; all enabled when omitted */
  services?: AnchorServices;
  /** Optional health status per service for degraded-state display */
  healthStatus?: AnchorHealthStatus;
}

import './themes.css';

// ─── Constants ────────────────────────────────────────────────────────────────

const KYC_META: Record<
  KYCLevel,
  { label: string; color: string; bg: string; border: string; desc: string }
> = {
  none: {
    label: "No KYC",
    color: "var(--ak-kyc-none-color)",
    bg: "var(--ak-kyc-none-bg)",
    border: "var(--ak-kyc-none-border)",
    desc: "No identity verification required.",
  },
  basic: {
    label: "Basic KYC",
    color: "var(--ak-kyc-basic-color)",
    bg: "var(--ak-kyc-basic-bg)",
    border: "var(--ak-kyc-basic-border)",
    desc: "Name and email required.",
  },
  full: {
    label: "Full KYC",
    color: "var(--ak-kyc-full-color)",
    bg: "var(--ak-kyc-full-bg)",
    border: "var(--ak-kyc-full-border)",
    desc: "Government ID and address verification.",
  },
  enhanced: {
    label: "Enhanced",
    color: "var(--ak-kyc-enhanced-color)",
    bg: "var(--ak-kyc-enhanced-bg)",
    border: "var(--ak-kyc-enhanced-border)",
    desc: "Full KYC plus source-of-funds documentation.",
  },
};

const TABS = ["assets", "fees", "limits", "kyc"] as const;
type Tab = (typeof TABS)[number];

const TAB_LABELS: Record<Tab, string> = {
  assets: "Assets",
  fees: "Fees",
  limits: "Limits",
  kyc: "KYC",
};

// ─── Formatters ───────────────────────────────────────────────────────────────

const fmt = (n: number, currency: string) =>
  `${currency} ${n.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;

function fmtFee(fee?: AssetFee): string {
  if (!fee) return "—";
  if (fee.type === "flat") return fmt(fee.flatAmount ?? 0, fee.currency);
  if (fee.type === "percent")
    return `${Number(fee.percent).toFixed(2)}% ${fee.currency}`;
  if (fee.type === "tiered" && fee.tiers) return "Tiered";
  return "—";
}

// ─── Micro-components ─────────────────────────────────────────────────────────

function AssetAvatar({
  asset,
  accent,
}: {
  asset: SupportedAsset;
  accent: string;
}) {
  if (asset.icon) return <span style={{ fontSize: 20 }}>{asset.icon}</span>;
  return (
    <div
      style={{
        width: 36,
        height: 36,
        borderRadius: 10,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: `linear-gradient(135deg, ${accent}22, ${accent}10)`,
        border: `1px solid ${accent}30`,
        fontFamily: "'Sora', sans-serif",
        fontWeight: 700,
        fontSize: 13,
        color: accent,
        letterSpacing: "-0.02em",
      }}
    >
      {asset.code.slice(0, 2)}
    </div>
  );
}

function OpBadge({ op }: { op: OperationType }) {
  const styles: Record<
    OperationType,
    { bg: string; color: string; label: string }
  > = {
    deposit:    { bg: "var(--ak-method-get-bg)",    color: "var(--ak-method-get-color)",    label: "Deposit"  },
    withdrawal: { bg: "var(--ak-status-failed-bg)", color: "var(--ak-status-failed-color)", label: "Withdraw" },
    both:       { bg: "var(--ak-method-patch-bg)",  color: "var(--ak-method-patch-color)",  label: "Both"     },
  };
  const s = styles[op];
  return (
    <span
      style={{
        fontSize: 10,
        fontFamily: "'Sora', sans-serif",
        fontWeight: 600,
        letterSpacing: "0.04em",
        padding: "2px 8px",
        borderRadius: 20,
        background: s.bg,
        color: s.color,
      }}
    >
      {s.label}
    </span>
  );
}

function KYCBadge({ level }: { level: KYCLevel }) {
  const m = KYC_META[level];
  return (
    <span
      style={{
        fontSize: 10,
        fontFamily: "'Sora', sans-serif",
        fontWeight: 700,
        letterSpacing: "0.06em",
        padding: "3px 9px",
        borderRadius: 20,
        background: m.bg,
        color: m.color,
        border: `1px solid ${m.border}`,
      }}
    >
      {m.label}
    </span>
  );
}

function RowDivider() {
  return (
    <div
      style={{
        height: 1,
        background: "linear-gradient(90deg, transparent, var(--ak-border), var(--ak-border), transparent)",
        margin: "2px 0",
        opacity: 0.5,
      }}
    />
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        fontFamily: "'Sora', sans-serif",
        fontSize: 9,
        fontWeight: 700,
        letterSpacing: "0.18em",
        textTransform: "uppercase",
        color: "var(--ak-text-muted)",
        marginBottom: 12,
      }}
    >
      {children}
    </div>
  );
}

function DataRow({
  label,
  value,
  mono,
  accent,
  children,
}: {
  label: string;
  value?: string | React.ReactNode;
  mono?: boolean;
  accent?: string;
  children?: React.ReactNode;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "9px 0",
        gap: 16,
      }}
    >
      <span
        style={{
          fontFamily: "'Sora', sans-serif",
          fontSize: 12,
          color: "var(--ak-text-muted)",
          flexShrink: 0,
        }}
      >
        {label}
      </span>
      <span
        style={{
          fontFamily: mono
            ? "'Source Code Pro', monospace"
            : "'Sora', sans-serif",
          fontSize: 12,
          fontWeight: 600,
          color: accent ?? "var(--ak-text)",
          textAlign: "right",
        }}
      >
        {children ?? value}
      </span>
    </div>
  );
}

// ─── Tab Panels ───────────────────────────────────────────────────────────────

function AssetsPanel({
  assets,
  accent,
  onSelect,
  selected,
}: {
  assets: SupportedAsset[];
  accent: string;
  onSelect: (code: string) => void;
  selected: string;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {assets.map((asset, i) => {
        const isActive = asset.code === selected;
        return (
          <button
            key={asset.code}
            onClick={() => onSelect(asset.code)}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
              padding: "12px 14px",
              borderRadius: 12,
              border: "none",
              background: isActive ? `${accent}0e` : "transparent",
              outline: isActive
                ? `1.5px solid ${accent}30`
                : "1.5px solid transparent",
              cursor: "pointer",
              textAlign: "left",
              transition: "all 0.18s",
              animation: `cap-slide-in 0.3s ease ${i * 0.06}s both`,
            }}
          >
            <AssetAvatar asset={asset} accent={isActive ? accent : "var(--ak-text-muted)"} />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div
                style={{
                  fontFamily: "'Sora', sans-serif",
                  fontSize: 13,
                  fontWeight: 700,
                  color: isActive ? "var(--ak-text)" : "var(--ak-text-muted)",
                }}
              >
                {asset.code}
                <span
                  style={{
                    fontWeight: 400,
                    color: "var(--ak-text-muted)",
                    marginLeft: 6,
                    fontSize: 12,
                  }}
                >
                  {asset.name}
                </span>
              </div>
              <div
                style={{
                  display: "flex",
                  gap: 5,
                  marginTop: 4,
                  flexWrap: "wrap",
                }}
              >
                {asset.depositEnabled && <OpBadge op="deposit" />}
                {asset.withdrawalEnabled && <OpBadge op="withdrawal" />}
              </div>
            </div>
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "flex-end",
                gap: 4,
              }}
            >
              {asset.networks?.slice(0, 2).map((n) => (
                <span
                  key={n}
                  style={{
                    fontFamily: "'Source Code Pro', monospace",
                    fontSize: 9,
                    fontWeight: 600,
                    color: "var(--ak-text-muted)",
                    background: "var(--ak-surface-2)",
                    padding: "2px 6px",
                    borderRadius: 4,
                  }}
                >
                  {n}
                </span>
              ))}
              {asset.networks && asset.networks.length > 2 && (
                <span
                  title={asset.networks.slice(2).join(", ")}
                  style={{
                    fontFamily: "'Source Code Pro', monospace",
                    fontSize: 9,
                    fontWeight: 600,
                    color: "var(--ak-text-muted)",
                    background: "var(--ak-surface-2)",
                    padding: "2px 6px",
                    borderRadius: 4,
                    cursor: "help",
                  }}
                >
                  +{asset.networks.length - 2}
                </span>
              )}
            </div>
            <div
              style={{
                color: isActive ? accent : "var(--ak-text-subtle)",
                fontSize: 14,
                transition: "color 0.2s",
              }}
            >
              ›
            </div>
          </button>
        );
      })}
    </div>
  );
}

function FeesPanel({
  asset,
  accent,
}: {
  asset: SupportedAsset;
  accent: string;
}) {
  const df = asset.fees.deposit;
  const wf = asset.fees.withdrawal;
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
      {/* Deposit fees */}
      {df && (
        <div>
          <SectionLabel>Deposit Fees</SectionLabel>
          <div
            style={{
              background: "var(--ak-surface-2)",
              borderRadius: 12,
              padding: "4px 14px",
              border: "1px solid var(--ak-border)",
            }}
          >
            <DataRow
              label="Type"
              value={df.type.charAt(0).toUpperCase() + df.type.slice(1)}
            />
            <RowDivider />
            {df.type === "flat" && (
              <DataRow
                label="Amount"
                value={fmt(df.flatAmount ?? 0, df.currency)}
                mono
                accent={accent}
              />
            )}
            {df.type === "percent" && (
              <DataRow
                label="Rate"
                value={`${Number(df.percent).toFixed(2)}% ${df.currency}`}
                mono
                accent={accent}
              />
            )}
            {df.type === "tiered" && df.tiers && (
              <div style={{ padding: "6px 0" }}>
                <div
                  style={{
                    fontFamily: "'Sora', sans-serif",
                    fontSize: 12,
                    color: "var(--ak-text-muted)",
                    marginBottom: 8,
                  }}
                >
                  Tiers
                </div>
                {df.tiers.map((t, i) => (
                  <div
                    key={i}
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      padding: "5px 0",
                      fontFamily: "'Source Code Pro', monospace",
                      fontSize: 11,
                    }}
                  >
                    <span style={{ color: "var(--ak-text-muted)" }}>
                      {t.upTo ? `up to ${fmt(t.upTo, df.currency)}` : "above"}
                    </span>
                    <span style={{ color: accent, fontWeight: 600 }}>
                      {t.fee}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Withdrawal fees */}
      {wf && (
        <div>
          <SectionLabel>Withdrawal Fees</SectionLabel>
          <div
            style={{
              background: "var(--ak-surface-2)",
              borderRadius: 12,
              padding: "4px 14px",
              border: "1px solid var(--ak-border)",
            }}
          >
            <DataRow
              label="Type"
              value={wf.type.charAt(0).toUpperCase() + wf.type.slice(1)}
            />
            <RowDivider />
            {wf.type === "flat" && (
              <DataRow
                label="Amount"
                value={fmt(wf.flatAmount ?? 0, wf.currency)}
                mono
                accent={accent}
              />
            )}
            {wf.type === "percent" && (
              <DataRow
                label="Rate"
                value={`${Number(wf.percent).toFixed(2)}% ${wf.currency}`}
                mono
                accent={accent}
              />
            )}
            {wf.type === "tiered" && wf.tiers && (
              <div style={{ padding: "6px 0" }}>
                {wf.tiers.map((t, i) => (
                  <div
                    key={i}
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      padding: "5px 0",
                      fontFamily: "'Source Code Pro', monospace",
                      fontSize: 11,
                    }}
                  >
                    <span style={{ color: "var(--ak-text-muted)" }}>
                      {t.upTo ? `up to ${fmt(t.upTo, wf.currency)}` : "above"}
                    </span>
                    <span style={{ color: accent, fontWeight: 600 }}>
                      {t.fee}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
      {!df && !wf && (
        <div
          style={{
            textAlign: "center",
            color: "var(--ak-text-muted)",
            fontFamily: "'Sora', sans-serif",
            fontSize: 13,
            padding: "24px 0",
          }}
        >
          No fee data available
        </div>
      )}
    </div>
  );
}

function LimitsPanel({
  asset,
  accent,
}: {
  asset: SupportedAsset;
  accent: string;
}) {
  const L = asset.limits;
  const currency = L.currency;

  const rows = [
    {
      label: "Min Deposit",
      value: L.minDeposit != null ? fmt(L.minDeposit, currency) : null,
    },
    {
      label: "Max Deposit",
      value: L.maxDeposit != null ? fmt(L.maxDeposit, currency) : null,
    },
    {
      label: "Min Withdrawal",
      value: L.minWithdrawal != null ? fmt(L.minWithdrawal, currency) : null,
    },
    {
      label: "Max Withdrawal",
      value: L.maxWithdrawal != null ? fmt(L.maxWithdrawal, currency) : null,
    },
    {
      label: "Daily Limit",
      value: L.dailyLimit != null ? fmt(L.dailyLimit, currency) : null,
    },
    {
      label: "Monthly Limit",
      value: L.monthlyLimit != null ? fmt(L.monthlyLimit, currency) : null,
    },
  ].filter((r) => r.value !== null);

  // Build deposit/withdrawal limit bars
  const depositUtilPct = L.maxDeposit != null && L.maxDeposit > 0
    ? Math.min(100, Math.max(0, ((L.minDeposit ?? 0) / L.maxDeposit) * 100))
    : 0;
  const wdUtilPct = L.maxWithdrawal != null && L.maxWithdrawal > 0
    ? Math.min(100, Math.max(0, ((L.minWithdrawal ?? 0) / L.maxWithdrawal) * 100))
    : 0;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
      {/* Visual range bars */}
      {L.maxDeposit != null && (
        <div>
          <SectionLabel>Deposit Range</SectionLabel>
          <div
            style={{
              background: "var(--ak-surface-2)",
              borderRadius: 12,
              padding: "14px 16px",
              border: "1px solid var(--ak-border)",
            }}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                marginBottom: 10,
                fontFamily: "'Source Code Pro', monospace",
                fontSize: 11,
              }}
            >
              <span style={{ color: "var(--ak-text-muted)" }}>
                {fmt(L.minDeposit ?? 0, currency)}
              </span>
              <span style={{ color: "var(--ak-text-muted)" }}>
                {fmt(L.maxDeposit!, currency)}
              </span>
            </div>
            <div
              role="progressbar"
              aria-label="Deposit limit range"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={depositUtilPct}
              style={{
                height: 6,
                borderRadius: 3,
                background: "var(--ak-border)",
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  height: "100%",
                  width: `${depositUtilPct}%`,
                  borderRadius: 3,
                  background: `linear-gradient(90deg, ${accent}40, ${accent})`,
                }}
              />
            </div>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                marginTop: 6,
                fontFamily: "'Sora', sans-serif",
                fontSize: 10,
                color: "var(--ak-text-muted)",
              }}
            >
              <span>Min</span>
              <span>Max</span>
            </div>
          </div>
      </div>
    )}

      {L.maxWithdrawal != null && (
        <div>
          <SectionLabel>Withdrawal Range</SectionLabel>
          <div
            style={{
              background: "var(--ak-surface-2)",
              borderRadius: 12,
              padding: "14px 16px",
              border: "1px solid var(--ak-border)",
            }}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                marginBottom: 10,
                fontFamily: "'Source Code Pro', monospace",
                fontSize: 11,
              }}
            >
              <span style={{ color: "var(--ak-text-muted)" }}>
                {fmt(L.minWithdrawal ?? 0, currency)}
              </span>
              <span style={{ color: "var(--ak-text-muted)" }}>
                {fmt(L.maxWithdrawal, currency)}
              </span>
            </div>
            <div
              role="progressbar"
              aria-label="Withdrawal limit range"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={wdUtilPct}
              style={{
                height: 6,
                borderRadius: 3,
                background: "var(--ak-border)",
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  height: "100%",
                  width: `${wdUtilPct}%`,
                  borderRadius: 3,
                  background: `linear-gradient(90deg, ${accent}40, ${accent})`,
                }}
              />
            </div>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                marginTop: 6,
                fontFamily: "'Sora', sans-serif",
                fontSize: 10,
                color: "var(--ak-text-muted)",
              }}
            >
              <span>Min</span>
              <span>Max</span>
            </div>
          </div>
        </div>
      )}

      {/* Table */}
      <div>
        <SectionLabel>All Limits</SectionLabel>
        <div
          style={{
            background: "var(--ak-surface-2)",
            borderRadius: 12,
            padding: "4px 14px",
            border: "1px solid var(--ak-border)",
          }}
        >
          {rows.map((row, i) => (
            <div key={row.label}>
              <DataRow
                label={row.label}
                value={row.value!}
                mono
                accent={accent}
              />
              {i < rows.length - 1 && <RowDivider />}
            </div>
          ))}
          {rows.length === 0 && (
            <div
              style={{
                textAlign: "center",
                color: "var(--ak-text-muted)",
                padding: "16px 0",
                fontFamily: "'Sora', sans-serif",
                fontSize: 13,
              }}
            >
              No limits configured
            </div>
          )}
        </div>
      </div>

      {/* Country coverage */}
      {asset.countries && asset.countries.length > 0 && (
        <div>
          <SectionLabel>Available In</SectionLabel>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
            {asset.countries.map((c) => (
              <span
                key={c}
                style={{
                  fontFamily: "'Source Code Pro', monospace",
                  fontSize: 11,
                  fontWeight: 600,
                  padding: "4px 10px",
                  borderRadius: 6,
                  background: "var(--ak-surface-2)",
                  color: "var(--ak-text-muted)",
                  border: "1px solid var(--ak-border)",
                }}
              >
                {c}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function KYCPanel({
  asset,
  accent,
}: {
  asset: SupportedAsset;
  accent: string;
}) {
  const kyc = asset.kyc;
  const m = KYC_META[kyc.level];
  const required = kyc.fields.filter((f) => f.required);
  const optional = kyc.fields.filter((f) => !f.required);

  // Document type icons mapping
  const docTypeIcons: Record<string, string> = {
    passport: "🛂",
    drivers_license: "🪪",
    national_id: "🆔",
    utility_bill: "📄",
    bank_statement: "🏦",
    proof_of_address: "🏠",
    tax_document: "📋",
    birth_certificate: "📜",
    social_security: "🔢",
  };

  // Document type labels
  const docTypeLabels: Record<string, string> = {
    passport: "Passport",
    drivers_license: "Driver's License",
    national_id: "National ID Card",
    utility_bill: "Utility Bill",
    bank_statement: "Bank Statement",
    proof_of_address: "Proof of Address",
    tax_document: "Tax Document",
    birth_certificate: "Birth Certificate",
    social_security: "Social Security Card",
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
      {/* Level banner */}
      <div
        style={{
          borderRadius: 12,
          padding: "16px 18px",
          background: m.bg,
          border: `1px solid ${m.border}`,
          display: "flex",
          alignItems: "center",
          gap: 16,
        }}
      >
        <div
          style={{
            width: 44,
            height: 44,
            borderRadius: 12,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            background: `${m.color}18`,
            border: `1.5px solid ${m.color}30`,
            fontSize: 20,
          }}
        >
          {kyc.level === "none"
            ? "✓"
            : kyc.level === "basic"
              ? "◎"
              : kyc.level === "full"
                ? "⬡"
                : "◈"}
        </div>
        <div>
          <div
            style={{
              fontFamily: "'Sora', sans-serif",
              fontWeight: 700,
              fontSize: 14,
              color: m.color,
            }}
          >
            {m.label}
          </div>
          <div
            style={{
              fontFamily: "'Sora', sans-serif",
              fontSize: 12,
              color: "var(--ak-text-muted)",
              marginTop: 2,
            }}
          >
            {kyc.description ?? m.desc}
          </div>
          {kyc.estimatedTime && (
            <div
              style={{
                fontFamily: "'Source Code Pro', monospace",
                fontSize: 10,
                color: "var(--ak-text-muted)",
                marginTop: 4,
              }}
            >
              Estimated: {kyc.estimatedTime}
            </div>
          )}
        </div>
      </div>

      {/* Document types required */}
      {kyc.documentTypes && kyc.documentTypes.length > 0 && (
        <div>
          <SectionLabel>Required Documents</SectionLabel>
          <div
            role="list"
            aria-label="Required document types"
            style={{ display: "flex", flexDirection: "column", gap: 8 }}
          >
            {kyc.documentTypes.map((docType) => (
              <div
                key={docType}
                role="listitem"
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 12,
                  padding: "12px 14px",
                  borderRadius: 10,
                  background: "#f8fafc",
                  border: "1px solid #e2e8f0",
                }}
              >
                <div
                  style={{
                    width: 36,
                    height: 36,
                    borderRadius: 8,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    background: `${m.color}10`,
                    border: `1px solid ${m.color}20`,
                    fontSize: 18,
                    flexShrink: 0,
                  }}
                >
                  {docTypeIcons[docType] ?? "📄"}
                </div>
                <div style={{ flex: 1 }}>
                  <div
                    style={{
                      fontFamily: "'Sora', sans-serif",
                      fontSize: 13,
                      fontWeight: 600,
                      color: "#1e293b",
                    }}
                  >
                    {docTypeLabels[docType] ?? docType.replace(/_/g, " ").replace(/\b\w/g, (l) => l.toUpperCase())}
                  </div>
                  <div
                    style={{
                      fontFamily: "'Source Code Pro', monospace",
                      fontSize: 10,
                      color: "#94a3b8",
                      marginTop: 2,
                    }}
                  >
                    {docType}
                  </div>
                </div>
                <div
                  style={{
                    padding: "3px 8px",
                    borderRadius: 20,
                    background: `${m.color}15`,
                    color: m.color,
                    fontSize: 9,
                    fontWeight: 700,
                    letterSpacing: "0.06em",
                  }}
                >
                  REQUIRED
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Required fields */}
      {required.length > 0 && (
        <div>
          <SectionLabel>Required Fields</SectionLabel>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {required.map((f) => (
              <div
                key={f.name}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 10,
                  padding: "9px 14px",
                  borderRadius: 9,
                  background: "var(--ak-surface-2)",
                  border: "1px solid var(--ak-border)",
                }}
              >
                <div
                  style={{
                    width: 6,
                    height: 6,
                    borderRadius: "50%",
                    background: m.color,
                    flexShrink: 0,
                  }}
                />
                <span
                  style={{
                    fontFamily: "'Sora', sans-serif",
                    fontSize: 12,
                    color: "var(--ak-text)",
                    flex: 1,
                  }}
                >
                  {f.label}
                </span>
                <span
                  style={{
                    fontFamily: "'Source Code Pro', monospace",
                    fontSize: 9,
                    color: "var(--ak-text-muted)",
                    background: "var(--ak-surface-2)",
                    padding: "2px 6px",
                    borderRadius: 4,
                  }}
                >
                  {f.name}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Optional fields */}
      {optional.length > 0 && (
        <div>
          <SectionLabel>Optional Fields</SectionLabel>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
            {optional.map((f) => (
              <span
                key={f.name}
                style={{
                  fontFamily: "'Sora', sans-serif",
                  fontSize: 11,
                  fontWeight: 500,
                  padding: "5px 12px",
                  borderRadius: 20,
                  background: "var(--ak-surface-2)",
                  color: "var(--ak-text-muted)",
                  border: "1px solid var(--ak-border)",
                }}
              >
                {f.label}
              </span>
            ))}
          </div>
        </div>
      )}

      {kyc.level === "none" && (
        <div style={{ textAlign: "center", padding: "16px 0" }}>
          <div style={{ fontSize: 28, marginBottom: 8 }}>🎉</div>
          <div
            style={{
              fontFamily: "'Sora', sans-serif",
              fontSize: 13,
              color: "var(--ak-kyc-none-color)",
              fontWeight: 600,
            }}
          >
            No verification needed
          </div>
          <div
            style={{
              fontFamily: "'Sora', sans-serif",
              fontSize: 12,
              color: "var(--ak-text-muted)",
              marginTop: 4,
            }}
          >
            Transact immediately after connecting your wallet.
          </div>
        </div>
      )}

      {/* Graceful fallback when no document types specified */}
      {kyc.level !== "none" && (!kyc.documentTypes || kyc.documentTypes.length === 0) && required.length === 0 && (
        <div
          style={{
            textAlign: "center",
            padding: "16px 0",
            color: "#94a3b8",
            fontFamily: "'Sora', sans-serif",
            fontSize: 12,
          }}
        >
          Document requirements will be provided during the verification process.
        </div>
      )}
    </div>
  );
}

// ─── Main Card ────────────────────────────────────────────────────────────────

// ─── Service Health Badge ─────────────────────────────────────────────────────

const HEALTH_META: Record<ServiceHealth, { label: string; color: string; bg: string; border: string; icon: string }> = {
  healthy:     { label: "Healthy",     color: "#22c55e", bg: "#f0fdf4", border: "#bbf7d0", icon: "●" },
  degraded:    { label: "Degraded",    color: "#f59e0b", bg: "#fffbeb", border: "#fde68a", icon: "◐" },
  unavailable: { label: "Unavailable", color: "#ef4444", bg: "#fef2f2", border: "#fecaca", icon: "○" },
};

function ServiceHealthBadge({ service, status }: { service: string; status: ServiceHealth }) {
  const m = HEALTH_META[status];
  return (
    <span style={{
      display: "inline-flex", alignItems: "center", gap: 4,
      padding: "3px 9px", borderRadius: 20,
      background: m.bg, border: `1px solid ${m.border}`,
      fontSize: 10, fontWeight: 600, color: m.color,
      fontFamily: "'Sora', sans-serif",
    }}>
      {m.icon} {service}
    </span>
  );
}

export function AnchorCapabilityCard({
  anchorName,
  domain,
  logoInitials,
  accentColor = "#3b82f6",
  description,
  assets,
  isLoading = false,
  error,
  services,
  healthStatus,
}: AnchorCapabilityCardProps) {
  const isDark = useTheme();
  const [activeTab, setActiveTab] = useState<Tab>("assets");
  const [selectedAssetCode, setSelectedAssetCode] = useState(
    assets[0]?.code ?? "",
  );

  const selectedAsset =
    assets.find((a) => a.code === selectedAssetCode) ?? assets[0];
  const accent = accentColor;

  // Derive whether any service is enabled
  const hasAnyService = !services ||
    services.deposits || services.withdrawals || services.quotes || services.kyc;

  const handleAssetSelect = (code: string) => {
    setSelectedAssetCode(code);
    // When selecting from asset list, jump to fees view for that asset
    setActiveTab("fees");
  };

  return (
    <div
      data-theme={isDark ? "dark" : "light"}
      style={{
        fontFamily: "'Sora', sans-serif",
        background: "var(--ak-surface)",
        borderRadius: 20,
        border: "1px solid var(--ak-border)",
        boxShadow:
          "0 8px 40px rgba(15,23,42,0.08), 0 2px 8px rgba(15,23,42,0.04)",
        overflow: "hidden",
        width: "100%",
        maxWidth: 460,
      }}
    >
      <style>{`
        @keyframes cap-slide-in { from { opacity:0; transform:translateY(8px); } to { opacity:1; transform:translateY(0); } }
        @keyframes cap-fade-in  { from { opacity:0; } to { opacity:1; } }
      `}</style>

      {/* ── Card Header ── */}
      <div
        style={{
          padding: "22px 24px 18px",
          background: "var(--ak-surface-2)",
          position: "relative",
          overflow: "hidden",
        }}
      >
        {/* Decorative diagonal stripe */}
        <div
          style={{
            position: "absolute",
            top: -40,
            right: -40,
            width: 140,
            height: 140,
            borderRadius: "50%",
            background: `${accent}12`,
            border: `1px solid ${accent}20`,
            pointerEvents: "none",
          }}
        />
        <div
          style={{
            position: "absolute",
            top: 20,
            right: 60,
            width: 60,
            height: 60,
            borderRadius: "50%",
            background: `${accent}08`,
            pointerEvents: "none",
          }}
        />

        <div
          style={{
            display: "flex",
            alignItems: "flex-start",
            gap: 14,
            position: "relative",
          }}
        >
          {/* Logo */}
          <div
            style={{
              width: 48,
              height: 48,
              borderRadius: 14,
              flexShrink: 0,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              background: `linear-gradient(135deg, ${accent}30, ${accent}18)`,
              border: `1.5px solid ${accent}40`,
              fontWeight: 800,
              fontSize: 16,
              color: accent,
              letterSpacing: "-0.03em",
              boxShadow: `0 0 20px ${accent}20`,
            }}
          >
            {logoInitials ?? anchorName.slice(0, 2).toUpperCase()}
          </div>

          <div style={{ flex: 1, minWidth: 0 }}>
            <div
              style={{
                fontSize: 17,
                fontWeight: 700,
                color: "var(--ak-text)",
                letterSpacing: "-0.02em",
                lineHeight: 1.2,
              }}
            >
              {anchorName}
            </div>
            <div
              style={{
                fontFamily: "'Source Code Pro', monospace",
                fontSize: 10,
                color: `${accent}cc`,
                marginTop: 3,
                letterSpacing: "0.04em",
              }}
            >
              {domain}
            </div>
            {description && (
              <div
                style={{
                  fontSize: 11,
                  color: "var(--ak-text-muted)",
                  marginTop: 6,
                  lineHeight: 1.5,
                }}
              >
                {description}
              </div>
            )}
          </div>

          {/* Asset count pill */}
          <div
            style={{
              flexShrink: 0,
              padding: "4px 10px",
              borderRadius: 20,
              background: `${accent}20`,
              border: `1px solid ${accent}30`,
              fontFamily: "'Source Code Pro', monospace",
              fontSize: 10,
              fontWeight: 600,
              color: accent,
            }}
          >
            {assets.length} asset{assets.length !== 1 ? "s" : ""}
          </div>
        </div>

        {/* KYC summary strips at bottom of header */}
        <div
          style={{ display: "flex", gap: 6, marginTop: 16, flexWrap: "wrap" }}
        >
          {Array.from(new Set(assets.map((a) => a.kyc.level))).map((level) => (
            <KYCBadge key={level} level={level} />
          ))}
          {Array.from(new Set(assets.flatMap((a) => a.networks ?? [])))
            .slice(0, 4)
            .map((n) => (
              <span
                key={n}
                style={{
                  fontFamily: "'Source Code Pro', monospace",
                  fontSize: 9,
                  fontWeight: 600,
                  padding: "3px 8px",
                  borderRadius: 20,
                  background: "rgba(255,255,255,0.06)",
                  color: "var(--ak-text-muted)",
                  border: "1px solid rgba(255,255,255,0.1)",
                }}
              >
                {n}
              </span>
            ))}
        </div>
      </div>

      {/* ── Asset selector strip (when not on assets tab) ── */}
      {activeTab !== "assets" && (
        <div
          style={{
            padding: "10px 24px 0",
            background: "var(--ak-surface-2)",
            borderBottom: "1px solid var(--ak-border)",
          }}
        >
          <div
            style={{
              display: "flex",
              gap: 4,
              overflowX: "auto",
              paddingBottom: 10,
            }}
          >
            {assets.map((a) => (
              <button
                key={a.code}
                onClick={() => setSelectedAssetCode(a.code)}
                style={{
                  fontFamily: "'Source Code Pro', monospace",
                  fontSize: 11,
                  fontWeight: 600,
                  padding: "5px 12px",
                  borderRadius: 8,
                  border: "none",
                  background: a.code === selectedAssetCode ? accent : "var(--ak-border)",
                  color: a.code === selectedAssetCode ? "#fff" : "var(--ak-text-muted)",
                  cursor: "pointer",
                  transition: "all 0.15s",
                  flexShrink: 0,
                  boxShadow:
                    a.code === selectedAssetCode
                      ? `0 2px 8px ${accent}40`
                      : "none",
                }}
              >
                {a.code}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* ── Tabs ── */}
      <div
        role="tablist"
        aria-label="Capability panels"
        style={{
          display: "flex",
          borderBottom: "1px solid var(--ak-border)",
          background: "var(--ak-surface-3)",
        }}
      >
        {TABS.map((tab, idx, arr) => (
          <button
            key={tab}
            id={`cap-tab-${tab}`}
            role="tab"
            aria-selected={activeTab === tab}
            aria-controls={`cap-tabpanel-${tab}`}
            tabIndex={activeTab === tab ? 0 : -1}
            onClick={() => setActiveTab(tab)}
            onKeyDown={(e) => {
              if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
                const next = e.key === "ArrowRight"
                  ? arr[(idx + 1) % arr.length]
                  : arr[(idx - 1 + arr.length) % arr.length];
                setActiveTab(next);
                (e.currentTarget.parentElement?.querySelector(`[id="cap-tab-${next}"]`) as HTMLElement)?.focus();
              }
              if (e.key === "Home") { setActiveTab(arr[0]); (e.currentTarget.parentElement?.querySelector(`[id="cap-tab-${arr[0]}"]`) as HTMLElement)?.focus(); }
              if (e.key === "End") { setActiveTab(arr[arr.length - 1]); (e.currentTarget.parentElement?.querySelector(`[id="cap-tab-${arr[arr.length - 1]}"]`) as HTMLElement)?.focus(); }
            }}
            style={{
              flex: 1,
              padding: "12px 0",
              border: "none",
              background: "transparent",
              fontFamily: "'Sora', sans-serif",
              fontSize: 12,
              fontWeight: activeTab === tab ? 700 : 500,
              color: activeTab === tab ? accent : "var(--ak-text-muted)",
              cursor: "pointer",
              position: "relative",
              transition: "color 0.2s",
              letterSpacing: "0.02em",
            }}
          >
            {TAB_LABELS[tab]}
            {activeTab === tab && (
              <div
                style={{
                  position: "absolute",
                  bottom: 0,
                  left: "20%",
                  right: "20%",
                  height: 2,
                  borderRadius: "2px 2px 0 0",
                  background: accent,
                  boxShadow: `0 0 8px ${accent}60`,
                }}
              />
            )}
          </button>
        ))}
      </div>

      {/* ── Tab Content ── */}
      <div
        role="tabpanel"
        id={`cap-tabpanel-${activeTab}`}
        aria-labelledby={`cap-tab-${activeTab}`}
        style={{
          padding: "20px 20px 22px",
          minHeight: 280,
          animation: "cap-fade-in 0.22s ease",
        }}
        key={activeTab + selectedAssetCode}
      >
        {/* Loading skeleton */}
        {isLoading && (
          <div
            role="status"
            aria-busy="true"
            aria-label="Loading anchor capabilities"
            style={{ display: "flex", flexDirection: "column", gap: 12, padding: "8px 0" }}
          >
            {[100, 80, 60, 90, 70].map((w, i) => (
              <div key={i} aria-hidden="true" style={{
                height: 16, width: `${w}%`, borderRadius: 8,
                background: "linear-gradient(90deg, #e2e8f0 25%, #f1f5f9 50%, #e2e8f0 75%)",
                backgroundSize: "200% 100%",
                animation: "cap-shimmer 1.4s ease infinite",
              }} />
            ))}
            <style>{`@keyframes cap-shimmer { 0%{background-position:200% 0} 100%{background-position:-200% 0} }`}</style>
            <span style={{ position: "absolute", width: 1, height: 1, overflow: "hidden", clip: "rect(0,0,0,0)" }}>
              Loading anchor capabilities
            </span>
          </div>
        )}

        {/* Error state */}
        {!isLoading && error && (
          <div style={{
            display: "flex", flexDirection: "column", alignItems: "center",
            justifyContent: "center", gap: 12, minHeight: 200, textAlign: "center",
          }}>
            <div style={{ fontSize: 36 }}>⚠️</div>
            <div style={{ fontSize: 14, fontWeight: 600, color: "#ef4444" }}>
              Failed to load capabilities
            </div>
            <div style={{
              fontSize: 12, color: "#94a3b8", maxWidth: 280, lineHeight: 1.6,
              background: "#fef2f2", border: "1px solid #fecaca",
              borderRadius: 10, padding: "10px 14px",
            }}>
              {error}
            </div>
          </div>
        )}

        {/* No-services state */}
        {!isLoading && !error && !hasAnyService && (
          <div style={{
            display: "flex", flexDirection: "column", alignItems: "center",
            justifyContent: "center", gap: 12, minHeight: 200, textAlign: "center",
          }}>
            <div style={{ fontSize: 36 }}>🔌</div>
            <div style={{ fontSize: 14, fontWeight: 600, color: "#0f172a" }}>
              No services available
            </div>
            <div style={{ fontSize: 12, color: "#94a3b8", lineHeight: 1.6 }}>
              This anchor has no active services at this time.
            </div>
          </div>
        )}

        {/* Health status banner (shown when at least one service is degraded/unavailable) */}
        {!isLoading && !error && hasAnyService && healthStatus && (
          <div style={{
            display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 16,
          }}>
            {(["deposits", "withdrawals", "quotes", "kyc"] as const)
              .filter((svc) => services?.[svc] !== false && healthStatus[svc] && healthStatus[svc] !== "healthy")
              .map((svc) => (
                <ServiceHealthBadge
                  key={svc}
                  service={svc.charAt(0).toUpperCase() + svc.slice(1)}
                  status={healthStatus[svc]!}
                />
              ))}
          </div>
        )}

        {/* Normal tab panels */}
        {!isLoading && !error && hasAnyService && (
          <>
            {activeTab === "assets" && (
              <AssetsPanel
                assets={assets}
                accent={accent}
                onSelect={handleAssetSelect}
                selected={selectedAssetCode}
              />
            )}
            {activeTab === "fees" && selectedAsset && (
              <FeesPanel asset={selectedAsset} accent={accent} />
            )}
            {activeTab === "limits" && selectedAsset && (
              <LimitsPanel asset={selectedAsset} accent={accent} />
            )}
            {activeTab === "kyc" && selectedAsset && (
              <KYCPanel asset={selectedAsset} accent={accent} />
            )}
          </>
        )}
      </div>
    </div>
  );
}

// ─── Demo ─────────────────────────────────────────────────────────────────────

const DEMO_ANCHORS: AnchorCapabilityCardProps[] = [
  {
    anchorName: "MoneyGram",
    domain: "moneygram.stellar.org",
    logoInitials: "MG",
    accentColor: "#2563eb",
    description:
      "Global cash-in / cash-out network. Available at 350,000+ locations.",
    assets: [
      {
        code: "USDC",
        name: "USD Coin",
        icon: "💵",
        operationTypes: ["both"],
        depositEnabled: true,
        withdrawalEnabled: true,
        networks: ["Cash", "ACH", "Wire"],
        countries: ["US", "MX", "GB", "DE", "IN", "PH", "NG"],
        fees: {
          deposit: {
            type: "tiered",
            currency: "USD",
            tiers: [
              { upTo: 200, fee: "$1.99" },
              { upTo: 1000, fee: "$3.99" },
              { upTo: null, fee: "0.5%" },
            ],
          },
          withdrawal: {
            type: "tiered",
            currency: "USD",
            tiers: [
              { upTo: 200, fee: "$2.49" },
              { upTo: 1000, fee: "$4.99" },
              { upTo: null, fee: "0.6%" },
            ],
          },
        },
        limits: {
          minDeposit: 10,
          maxDeposit: 10000,
          minWithdrawal: 10,
          maxWithdrawal: 10000,
          dailyLimit: 10000,
          monthlyLimit: 50000,
          currency: "USD",
        },
        kyc: {
          level: "basic",
          estimatedTime: "< 2 minutes",
          fields: [
            { name: "first_name", label: "First Name", required: true },
            { name: "last_name", label: "Last Name", required: true },
            { name: "email_address", label: "Email Address", required: true },
            { name: "phone_number", label: "Phone Number", required: true },
            { name: "date_of_birth", label: "Date of Birth", required: false },
            { name: "address", label: "Home Address", required: false },
          ],
        },
      },
      {
        code: "XLM",
        name: "Stellar Lumens",
        icon: "⭐",
        operationTypes: ["deposit"],
        depositEnabled: true,
        withdrawalEnabled: false,
        networks: ["Cash"],
        countries: ["US", "MX"],
        fees: {
          deposit: { type: "flat", flatAmount: 0.99, currency: "USD" },
        },
        limits: { minDeposit: 5, maxDeposit: 500, currency: "USD" },
        kyc: {
          level: "none",
          fields: [],
        },
      },
    ],
  },
  {
    anchorName: "Vibrant",
    domain: "vibrant.stellar.org",
    logoInitials: "VB",
    accentColor: "#7c3aed",
    description: "SEPA & cross-border EUR transfers for European residents.",
    assets: [
      {
        code: "EURC",
        name: "Euro Coin",
        icon: "💶",
        operationTypes: ["both"],
        depositEnabled: true,
        withdrawalEnabled: true,
        networks: ["SEPA", "SWIFT"],
        countries: ["DE", "FR", "ES", "IT", "NL", "BE", "PT"],
        fees: {
          deposit: { type: "percent", percent: 0.1, currency: "EUR" },
          withdrawal: { type: "flat", flatAmount: 0.5, currency: "EUR" },
        },
        limits: {
          minDeposit: 10,
          maxDeposit: 50000,
          minWithdrawal: 10,
          maxWithdrawal: 50000,
          dailyLimit: 50000,
          monthlyLimit: 200000,
          currency: "EUR",
        },
        kyc: {
          level: "full",
          estimatedTime: "1 – 3 business days",
          description:
            "EU AML regulations require government-issued ID and address verification.",
          fields: [
            { name: "first_name", label: "First Name", required: true },
            { name: "last_name", label: "Last Name", required: true },
            { name: "id_type", label: "ID Document Type", required: true },
            { name: "id_number", label: "ID Number", required: true },
            { name: "address", label: "Home Address", required: true },
            { name: "date_of_birth", label: "Date of Birth", required: true },
            { name: "occupation", label: "Occupation", required: false },
            { name: "employer_name", label: "Employer Name", required: false },
          ],
        },
      },
      {
        code: "USDC",
        name: "USD Coin",
        icon: "💵",
        operationTypes: ["withdrawal"],
        depositEnabled: false,
        withdrawalEnabled: true,
        networks: ["SWIFT"],
        countries: ["US", "GB"],
        fees: {
          withdrawal: { type: "flat", flatAmount: 12.0, currency: "USD" },
        },
        limits: {
          minWithdrawal: 100,
          maxWithdrawal: 25000,
          dailyLimit: 25000,
          currency: "USD",
        },
        kyc: {
          level: "enhanced",
          estimatedTime: "3 – 5 business days",
          description:
            "Source-of-funds documentation required for wire transfers.",
          fields: [
            { name: "first_name", label: "First Name", required: true },
            { name: "last_name", label: "Last Name", required: true },
            { name: "id_type", label: "ID Document Type", required: true },
            { name: "id_number", label: "ID Number", required: true },
            { name: "address", label: "Home Address", required: true },
            { name: "date_of_birth", label: "Date of Birth", required: true },
            { name: "bank_statement", label: "Bank Statement", required: true },
            {
              name: "source_of_funds",
              label: "Source of Funds",
              required: true,
            },
            { name: "tax_id", label: "Tax ID", required: false },
          ],
        },
      },
    ],
  },
];

export default function AnchorCapabilityDemo() {
  const [activeAnchor, setActiveAnchor] = useState(0);

  return (
    <div
      style={{
        fontFamily: "'Sora', sans-serif",
        minHeight: "100vh",
        background:
          "linear-gradient(150deg, #f0f4ff 0%, #faf5ff 50%, #f0fdf4 100%)",
        padding: "48px 24px",
      }}
    >
      <style>{`
        * { box-sizing: border-box; margin: 0; padding: 0; }
      `}</style>

      <div style={{ maxWidth: 960, margin: "0 auto" }}>
        {/* Page header */}
        <div style={{ textAlign: "center", marginBottom: 48 }}>
          <div
            style={{
              display: "inline-block",
              fontFamily: "'Source Code Pro', monospace",
              fontSize: 10,
              letterSpacing: "0.2em",
              textTransform: "uppercase",
              color: "var(--ak-text-muted)",
              background: "var(--ak-surface-2)",
              padding: "4px 14px",
              borderRadius: 20,
              border: "1px solid var(--ak-border)",
              marginBottom: 16,
            }}
          >
            Component / AnchorCapabilityCard
          </div>
          <h1
            style={{
              fontFamily: "'Sora', sans-serif",
              fontSize: 36,
              fontWeight: 800,
              color: "var(--ak-text)",
              letterSpacing: "-0.03em",
              marginBottom: 12,
              lineHeight: 1.15,
            }}
          >
            Anchor Capability Card
          </h1>
          <p
            style={{
              fontSize: 14,
              color: "var(--ak-text-muted)",
              maxWidth: 420,
              margin: "0 auto",
              lineHeight: 1.65,
            }}
          >
            Displays supported assets, fees, transaction limits, and KYC
            requirements for any Stellar anchor.
          </p>
        </div>

        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr auto",
            gap: 48,
            alignItems: "start",
          }}
        >
          {/* Left — selector + info */}
          <div style={{ display: "flex", flexDirection: "column", gap: 28 }}>
            {/* Anchor switcher */}
            <div>
              <div
                style={{
                  fontFamily: "'Source Code Pro', monospace",
                  fontSize: 10,
                  letterSpacing: "0.16em",
                  color: "var(--ak-text-muted)",
                  textTransform: "uppercase",
                  marginBottom: 12,
                }}
              >
                Select Anchor
              </div>
              <div style={{ display: "flex", gap: 10 }}>
                {DEMO_ANCHORS.map((a, i) => (
                  <button
                    key={a.domain}
                    onClick={() => setActiveAnchor(i)}
                    style={{
                      flex: 1,
                      padding: "14px 18px",
                      borderRadius: 14,
                      border: "none",
                      background: activeAnchor === i ? a.accentColor! : "#fff",
                      color: activeAnchor === i ? "#fff" : "#64748b",
                      fontFamily: "'Sora', sans-serif",
                      fontSize: 13,
                      fontWeight: 600,
                      cursor: "pointer",
                      transition: "all 0.2s",
                      boxShadow:
                        activeAnchor === i
                          ? `0 4px 18px ${a.accentColor!}40`
                          : "0 1px 4px rgba(0,0,0,0.06), 0 0 0 1px #e2e8f0",
                    }}
                  >
                    {a.anchorName}
                    <div
                      style={{
                        fontSize: 10,
                        fontFamily: "'Source Code Pro', monospace",
                        opacity: 0.7,
                        marginTop: 3,
                      }}
                    >
                      {a.domain.replace(".stellar.org", "")}
                    </div>
                  </button>
                ))}
              </div>
            </div>

            {/* Props reference */}
            <div
              style={{
                background: "#0f172a",
                borderRadius: 16,
                padding: "20px 22px",
                border: "1px solid #1e293b",
              }}
            >
              <div
                style={{
                  fontFamily: "'Source Code Pro', monospace",
                  fontSize: 10,
                  letterSpacing: "0.16em",
                  color: "var(--ak-text-muted)",
                  textTransform: "uppercase",
                  marginBottom: 14,
                }}
              >
                Usage
              </div>
              <pre
                style={{
                  fontFamily: "'Source Code Pro', monospace",
                  fontSize: 11,
                  color: "var(--ak-text-muted)",
                  lineHeight: 1.8,
                  overflowX: "auto",
                  whiteSpace: "pre-wrap",
                }}
              >{`<AnchorCapabilityCard
  anchorName="MoneyGram"
  domain="moneygram.stellar.org"
  accentColor="#2563eb"
  assets={[
    {
      code: "USDC",
      name: "USD Coin",
      depositEnabled: true,
      withdrawalEnabled: true,
      fees: {
        deposit: {
          type: "tiered",
          currency: "USD",
          tiers: [...]
        }
      },
      limits: {
        minDeposit: 10,
        maxDeposit: 10000,
        dailyLimit: 10000,
        currency: "USD"
      },
      kyc: {
        level: "basic",
        fields: [...]
      }
    }
  ]}
/>`}</pre>
            </div>

            {/* KYC level legend */}
            <div
              style={{
                background: "var(--ak-surface)",
                borderRadius: 16,
                padding: "18px 20px",
                border: "1px solid var(--ak-border)",
                boxShadow: "0 2px 8px rgba(0,0,0,0.04)",
              }}
            >
              <div
                style={{
                  fontFamily: "'Source Code Pro', monospace",
                  fontSize: 10,
                  letterSpacing: "0.16em",
                  color: "var(--ak-text-muted)",
                  textTransform: "uppercase",
                  marginBottom: 14,
                }}
              >
                KYC Level Reference
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {(
                  Object.entries(KYC_META) as [
                    KYCLevel,
                    (typeof KYC_META)[KYCLevel],
                  ][]
                ).map(([level, m]) => (
                  <div
                    key={level}
                    style={{ display: "flex", alignItems: "center", gap: 10 }}
                  >
                    <KYCBadge level={level} />
                    <span style={{ fontSize: 12, color: "var(--ak-text-muted)" }}>
                      {m.desc}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Right — live card */}
          <div style={{ position: "sticky", top: 24, width: 460 }}>
            <div
              style={{
                fontFamily: "'Source Code Pro', monospace",
                fontSize: 10,
                letterSpacing: "0.16em",
                color: "var(--ak-text-muted)",
                textTransform: "uppercase",
                marginBottom: 12,
              }}
            >
              Preview
            </div>
            <AnchorCapabilityCard
              key={activeAnchor}
              {...DEMO_ANCHORS[activeAnchor]}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
