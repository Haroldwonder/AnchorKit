import { useState, useCallback, useMemo, useRef, useEffect } from "react";

// ─── Types ────────────────────────────────────────────────────────────────────

type JsonValue = string | number | boolean | null | JsonObject | JsonArray;
type JsonObject = { [key: string]: JsonValue };
type JsonArray = JsonValue[];

export type ViewerTheme = "ember" | "arctic" | "forest";
export type ViewerMode = "tree" | "raw";

export interface JsonViewerProps {
  data: JsonValue;
  title?: string;
  subtitle?: string;
  status?: number;
  responseTime?: number;
  theme?: ViewerTheme;
  defaultMode?: ViewerMode;
  defaultExpandDepth?: number;
  searchable?: boolean;
  className?: string;
}

// ─── Theme Definitions ────────────────────────────────────────────────────────

const THEMES: Record<
  ViewerTheme,
  {
    bg: string;
    surface: string;
    border: string;
    lineNum: string;
    gutter: string;
    key: string;
    str: string;
    num: string;
    bool: string;
    nil: string;
    punct: string;
    text: string;
    muted: string;
    accent: string;
    accentDim: string;
    matchBg: string;
    matchText: string;
    tab: string;
    tabActive: string;
    collapse: string;
    collapseHover: string;
  }
> = {
  ember: {
    bg: "#141210",
    surface: "#1c1916",
    border: "#2d2720",
    gutter: "#1a1714",
    lineNum: "#3d3530",
    key: "#f97316",
    str: "#86efac",
    num: "#7dd3fc",
    bool: "#fb923c",
    nil: "#6b7280",
    punct: "#57534e",
    text: "#e7e5e0",
    muted: "#78716c",
    accent: "#f97316",
    accentDim: "#f9731620",
    matchBg: "#f97316",
    matchText: "#141210",
    tab: "#1c1916",
    tabActive: "#252019",
    collapse: "#292420",
    collapseHover: "#332c26",
  },
  arctic: {
    bg: "#0d1117",
    surface: "#161b22",
    border: "#21262d",
    gutter: "#0d1117",
    lineNum: "#30363d",
    key: "#79c0ff",
    str: "#a5d6ff",
    num: "#ff7b72",
    bool: "#d2a8ff",
    nil: "#8b949e",
    punct: "#6e7681",
    text: "#c9d1d9",
    muted: "#6e7681",
    accent: "#58a6ff",
    accentDim: "#58a6ff20",
    matchBg: "#58a6ff",
    matchText: "#0d1117",
    tab: "#161b22",
    tabActive: "#1c2129",
    collapse: "#1c2129",
    collapseHover: "#21262d",
  },
  forest: {
    bg: "#0f1510",
    surface: "#141d15",
    border: "#1e2e1f",
    gutter: "#111a12",
    lineNum: "#2d4030",
    key: "#86efac",
    str: "#fde68a",
    num: "#67e8f9",
    bool: "#a78bfa",
    nil: "#6b7280",
    punct: "#4a6050",
    text: "#d1fae5",
    muted: "#6b7280",
    accent: "#4ade80",
    accentDim: "#4ade8020",
    matchBg: "#4ade80",
    matchText: "#0f1510",
    tab: "#141d15",
    tabActive: "#1a2a1b",
    collapse: "#1a2a1b",
    collapseHover: "#1f3320",
  },
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Detect circular references and replace them with [Circular] markers
 */
function removeCircularReferences(obj: any, seen = new WeakSet()): JsonValue {
  if (obj === null || typeof obj !== 'object') {
    return obj as JsonValue;
  }

  if (seen.has(obj)) {
    return '[Circular]' as any;
  }

  seen.add(obj);

  if (Array.isArray(obj)) {
    return obj.map(item => removeCircularReferences(item, seen)) as JsonArray;
  }

  const result: JsonObject = {};
  for (const key in obj) {
    if (Object.prototype.hasOwnProperty.call(obj, key)) {
      result[key] = removeCircularReferences(obj[key], seen);
    }
  }

  return result;
}

function getType(val: JsonValue): string {
  if (val === null) return "null";
  if (Array.isArray(val)) return "array";
  return typeof val;
}

function typeColor(type: string, t: (typeof THEMES)[ViewerTheme]): string {
  return (
    {
      string: t.str,
      number: t.num,
      boolean: t.bool,
      null: t.nil,
      object: t.key,
      array: t.key,
    }[type] ?? t.text
  );
}

function typeLabel(type: string): string {
  return (
    {
      string: "str",
      number: "num",
      boolean: "bool",
      null: "null",
      object: "obj",
      array: "arr",
    }[type] ?? type
  );
}

function countChildren(val: JsonValue): number {
  if (val === null || typeof val !== "object") return 0;
  return Array.isArray(val) ? val.length : Object.keys(val).length;
}

function highlight(
  text: string,
  query: string,
  accent: string,
  matchText: string,
): React.ReactNode {
  if (!query) return text;
  const regex = new RegExp(
    `(${query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`,
    "gi",
  );
  const parts = text.split(regex);
  return parts.map((p, i) =>
    regex.test(p) ? (
      <mark
        key={i}
        style={{
          background: accent,
          color: matchText,
          borderRadius: 2,
          padding: "0 1px",
        }}
      >
        {p}
      </mark>
    ) : (
      p
    ),
  );
}

function syntaxHighlightRaw(
  json: string,
  t: (typeof THEMES)[ViewerTheme],
): string {
  return json
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(
      /("(\\u[\da-fA-F]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+-]?\d+)?)/g,
      (m) => {
        let c = t.num;
        if (/^"/.test(m)) c = /:$/.test(m) ? t.key : t.str;
        else if (/true|false/.test(m)) c = t.bool;
        else if (/null/.test(m)) c = t.nil;
        return `<span style="color:${c}">${m}</span>`;
      },
    )
    .replace(/([{}[\],])/g, `<span style="color:${t.punct}">$1</span>`);
}

// ─── Tree Node ────────────────────────────────────────────────────────────────

interface TreeNodeProps {
  nodeKey?: string | number;
  value: JsonValue;
  depth: number;
  isLast: boolean;
  theme: (typeof THEMES)[ViewerTheme];
  defaultExpandDepth: number;
  search: string;
  path: string;
  expandedPaths: Set<string>;
  onToggle: (path: string) => void;
  lineRef: React.MutableRefObject<number>;
}

function TreeNode({
  nodeKey,
  value,
  depth,
  isLast,
  theme: t,
  defaultExpandDepth,
  search,
  path,
  expandedPaths,
  onToggle,
  lineRef,
}: TreeNodeProps) {
  const type = getType(value);
  const isExpandable = type === "object" || type === "array";
  const isExpanded = expandedPaths.has(path);
  const childCount = countChildren(value);
  const lineNum = ++lineRef.current;
  const indent = depth * 18;

  const keyStr = nodeKey !== undefined ? String(nodeKey) : null;
  const comma = isLast ? "" : ",";

  // Scalar display
  if (!isExpandable) {
    const raw = type === "string" ? `"${value}"` : String(value);
    const color = typeColor(type, t);
    const sqStr = search
      ? raw.toLowerCase().includes(search.toLowerCase())
      : false;
    const sqKey =
      search && keyStr
        ? keyStr.toLowerCase().includes(search.toLowerCase())
        : false;
    const highlighted = sqStr || sqKey;

    return (
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          minHeight: 22,
          background: highlighted ? `${t.accent}10` : "transparent",
          borderLeft: highlighted
            ? `2px solid ${t.accent}`
            : "2px solid transparent",
          transition: "background 0.15s",
        }}
      >
        {/* Gutter */}
        <div
          style={{
            width: 44,
            flexShrink: 0,
            textAlign: "right",
            paddingRight: 12,
            fontFamily: "monospace",
            fontSize: 10,
            color: t.lineNum,
            lineHeight: "22px",
            userSelect: "none",
            background: t.gutter,
          }}
        >
          {lineNum}
        </div>
        {/* Content */}
        <div
          style={{
            flex: 1,
            paddingLeft: indent + 4,
            paddingRight: 12,
            lineHeight: "22px",
            fontFamily: "'Fira Code', 'Cascadia Code', monospace",
            fontSize: 12,
            whiteSpace: "pre-wrap",
            wordBreak: "break-all",
          }}
        >
          {keyStr !== null && (
            <span style={{ color: t.key }}>
              {search
                ? highlight(`"${keyStr}"`, search, t.matchBg, t.matchText)
                : `"${keyStr}"`}
              <span style={{ color: t.punct }}>: </span>
            </span>
          )}
          <span style={{ color }}>
            {search ? highlight(raw, search, t.matchBg, t.matchText) : raw}
          </span>
          <span style={{ color: t.punct }}>{comma}</span>
        </div>
      </div>
    );
  }

  // Expandable (object/array)
  const bracket = type === "array" ? ["[", "]"] : ["{", "}"];
  const entries: [string | number, JsonValue][] =
    type === "array"
      ? (value as JsonArray).map((v, i) => [i, v])
      : Object.entries(value as JsonObject);

  return (
    <div>
      {/* Opening line */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          minHeight: 22,
          cursor: "pointer",
          background: "transparent",
          borderLeft: "2px solid transparent",
        }}
        onClick={() => onToggle(path)}
      >
        <div
          style={{
            width: 44,
            flexShrink: 0,
            textAlign: "right",
            paddingRight: 12,
            fontFamily: "monospace",
            fontSize: 10,
            color: t.lineNum,
            lineHeight: "22px",
            userSelect: "none",
            background: t.gutter,
          }}
        >
          {lineNum}
        </div>
        <div
          style={{
            flex: 1,
            paddingLeft: indent + 4,
            paddingRight: 12,
            lineHeight: "22px",
            fontFamily: "'Fira Code', 'Cascadia Code', monospace",
            fontSize: 12,
            display: "flex",
            alignItems: "center",
            gap: 6,
          }}
        >
          {/* Collapse toggle arrow */}
          <span
            style={{
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
              width: 14,
              height: 14,
              borderRadius: 3,
              flexShrink: 0,
              background: t.collapse,
              color: t.muted,
              fontSize: 8,
              transition: "transform 0.18s, background 0.15s",
              transform: isExpanded ? "rotate(90deg)" : "rotate(0deg)",
            }}
          >
            ▶
          </span>

          {keyStr !== null && (
            <span style={{ color: t.key }}>
              {search
                ? highlight(`"${keyStr}"`, search, t.matchBg, t.matchText)
                : `"${keyStr}"`}
              <span style={{ color: t.punct }}>: </span>
            </span>
          )}

          <span style={{ color: t.punct }}>{bracket[0]}</span>

          {!isExpanded && (
            <>
              <span
                style={{
                  fontFamily: "monospace",
                  fontSize: 10,
                  padding: "1px 7px",
                  borderRadius: 10,
                  background: t.accentDim,
                  color: t.accent,
                  letterSpacing: "0.04em",
                }}
              >
                {childCount} {type === "array" ? "items" : "keys"}
              </span>
              <span style={{ color: t.punct }}>
                {bracket[1]}
                {comma}
              </span>
            </>
          )}
        </div>
      </div>

      {/* Children */}
      {isExpanded && (
        <div
          style={{
            borderLeft: `1px solid ${t.border}`,
            marginLeft: 44 + indent + 4 + 7,
            animation: "jv-expand 0.18s ease",
          }}
        >
          {entries.map(([k, v], i) => (
            <TreeNode
              key={String(k)}
              nodeKey={k}
              value={v}
              depth={depth + 1}
              isLast={i === entries.length - 1}
              theme={t}
              defaultExpandDepth={defaultExpandDepth}
              search={search}
              path={`${path}.${k}`}
              expandedPaths={expandedPaths}
              onToggle={onToggle}
              lineRef={lineRef}
            />
          ))}
        </div>
      )}

      {/* Closing bracket */}
      {isExpanded && (
        <div
          style={{ display: "flex", alignItems: "flex-start", minHeight: 22 }}
        >
          <div
            style={{
              width: 44,
              flexShrink: 0,
              textAlign: "right",
              paddingRight: 12,
              fontFamily: "monospace",
              fontSize: 10,
              color: t.lineNum,
              lineHeight: "22px",
              userSelect: "none",
              background: t.gutter,
            }}
          >
            {++lineRef.current}
          </div>
          <div
            style={{
              flex: 1,
              paddingLeft: indent + 4,
              paddingRight: 12,
              lineHeight: "22px",
              fontFamily: "'Fira Code', 'Cascadia Code', monospace",
              fontSize: 12,
              color: t.punct,
            }}
          >
            {bracket[1]}
            {comma}
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Path Collector ───────────────────────────────────────────────────────────

function collectPaths(
  value: JsonValue,
  path: string,
  depth: number,
  maxDepth: number,
  acc: Set<string>,
) {
  const type = getType(value);
  if (depth > maxDepth) return;
  if (type === "object" || type === "array") {
    acc.add(path);
    const entries: [string | number, JsonValue][] =
      type === "array"
        ? (value as JsonArray).map((v, i) => [i, v])
        : Object.entries(value as JsonObject);
    entries.forEach(([k, v]) =>
      collectPaths(v, `${path}.${k}`, depth + 1, maxDepth, acc),
    );
  }
}

// ─── Status Badge ─────────────────────────────────────────────────────────────

function StatusBadge({ status }: { status?: number }) {
  if (!status) return null;
  const ok = status >= 200 && status < 300;
  const warn = status >= 300 && status < 500;
  const [bg, color] = ok
    ? ["#14532d", "#86efac"]
    : warn
      ? ["#451a03", "#fdba74"]
      : ["#450a0a", "#fca5a5"];
  return (
    <span
      style={{
        fontFamily: "monospace",
        fontSize: 10,
        fontWeight: 700,
        padding: "2px 8px",
        borderRadius: 4,
        background: bg,
        color,
        letterSpacing: "0.06em",
      }}
    >
      {ok ? "✓ " : warn ? "⚠ " : "✕ "}
      {status}
    </span>
  );
}

// ─── Main Component ───────────────────────────────────────────────────────────

export function JsonViewer({
  data,
  title,
  subtitle,
  status,
  responseTime,
  theme: themeName = "ember",
  defaultMode = "tree",
  defaultExpandDepth = 2,
  searchable = true,
}: JsonViewerProps) {
  const t = THEMES[themeName];
  const [mode, setMode] = useState<ViewerMode>(defaultMode);
  const [search, setSearch] = useState("");
  const [copied, setCopied] = useState(false);
  
  // Remove circular references before processing
  const safeData = useMemo(() => removeCircularReferences(data), [data]);
  
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(() => {
    const acc = new Set<string>();
    collectPaths(safeData, "root", 0, defaultExpandDepth, acc);
    return acc;
  });

  const json = useMemo(() => JSON.stringify(safeData, null, 2), [safeData]);
  const lineRef = useRef(0);

  // Count search matches in raw mode
  const matchCount = useMemo(() => {
    if (!search) return 0;
    return (
      json.match(
        new RegExp(search.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "gi"),
      ) ?? []
    ).length;
  }, [json, search]);

  const togglePath = useCallback((path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev);
      next.has(path) ? next.delete(path) : next.add(path);
      return next;
    });
  }, []);

  const expandAll = () => {
    const acc = new Set<string>();
    collectPaths(safeData, "root", 0, 99, acc);
    setExpandedPaths(acc);
  };

  const collapseAll = () => setExpandedPaths(new Set());

  const copy = () => {
    navigator.clipboard.writeText(json);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  // Reset line counter on each render
  lineRef.current = 0;

  const rawHighlighted = useMemo(() => syntaxHighlightRaw(json, t), [json, t]);
  const lineCount = json.split("\n").length;

  return (
    <div
      style={{
        fontFamily: "'Fira Code', monospace",
        background: t.bg,
        borderRadius: 14,
        border: `1px solid ${t.border}`,
        overflow: "hidden",
        boxShadow: "0 8px 48px rgba(0,0,0,0.5), 0 2px 8px rgba(0,0,0,0.4)",
      }}
    >
      <style>{`
        @import url('https://fonts.googleapis.com/css2?family=Fira+Code:wght@400;500;600&display=swap');
        @keyframes jv-expand { from { opacity:0; transform:translateY(-4px); } to { opacity:1; transform:translateY(0); } }
        @keyframes jv-pulse  { 0%,100%{opacity:1} 50%{opacity:0.4} }
        .jv-node-row:hover { background: rgba(255,255,255,0.025) !important; }
      `}</style>

      {/* ── Titlebar ── */}
      <div
        style={{
          padding: "10px 16px",
          background: t.surface,
          borderBottom: `1px solid ${t.border}`,
          display: "flex",
          alignItems: "center",
          gap: 10,
        }}
      >
        {/* Traffic lights */}
        <div style={{ display: "flex", gap: 6, flexShrink: 0 }}>
          {["#ff5f57", "#febc2e", "#28c840"].map((c) => (
            <div
              key={c}
              style={{
                width: 11,
                height: 11,
                borderRadius: "50%",
                background: c,
                opacity: 0.8,
              }}
            />
          ))}
        </div>

        {/* Title */}
        <div style={{ flex: 1, minWidth: 0 }}>
          {title && (
            <div
              style={{
                fontSize: 11,
                fontWeight: 600,
                color: t.text,
                letterSpacing: "0.03em",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {title}
            </div>
          )}
          {subtitle && (
            <div
              style={{
                fontSize: 9,
                color: t.muted,
                marginTop: 1,
                letterSpacing: "0.06em",
                fontFamily: "monospace",
              }}
            >
              {subtitle}
            </div>
          )}
        </div>

        {/* Metadata chips */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            flexShrink: 0,
          }}
        >
          <StatusBadge status={status} />
          {responseTime != null && (
            <span
              style={{ fontFamily: "monospace", fontSize: 10, color: t.muted }}
            >
              {responseTime}ms
            </span>
          )}
          <span
            style={{ fontFamily: "monospace", fontSize: 10, color: t.lineNum }}
          >
            {lineCount} lines
          </span>
        </div>
      </div>

      {/* ── Toolbar ── */}
      <div
        style={{
          padding: "8px 14px",
          background: t.surface,
          borderBottom: `1px solid ${t.border}`,
          display: "flex",
          alignItems: "center",
          gap: 8,
        }}
      >
        {/* Mode toggle */}
        <div
          style={{
            display: "flex",
            borderRadius: 7,
            overflow: "hidden",
            border: `1px solid ${t.border}`,
            flexShrink: 0,
          }}
        >
          {(["tree", "raw"] as ViewerMode[]).map((m) => (
            <button
              key={m}
              onClick={() => setMode(m)}
              style={{
                padding: "5px 12px",
                fontSize: 10,
                fontWeight: 600,
                letterSpacing: "0.08em",
                textTransform: "uppercase",
                fontFamily: "monospace",
                cursor: "pointer",
                border: "none",
                background: mode === m ? t.accent : t.tab,
                color: mode === m ? t.bg : t.muted,
                transition: "all 0.15s",
              }}
            >
              {m}
            </button>
          ))}
        </div>

        {/* Tree controls */}
        {mode === "tree" && (
          <>
            <button
              onClick={expandAll}
              style={{ ...toolBtn(t), borderColor: t.border }}
            >
              ⊞ Expand All
            </button>
            <button
              onClick={collapseAll}
              style={{ ...toolBtn(t), borderColor: t.border }}
            >
              ⊟ Collapse
            </button>
          </>
        )}

        {/* Search */}
        {searchable && (
          <div
            style={{
              flex: 1,
              display: "flex",
              alignItems: "center",
              gap: 6,
              background: t.bg,
              border: `1px solid ${t.border}`,
              borderRadius: 7,
              padding: "5px 10px",
            }}
          >
            <span style={{ color: t.muted, fontSize: 11 }}>⌕</span>
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search keys & values…"
              style={{
                flex: 1,
                background: "transparent",
                border: "none",
                outline: "none",
                fontFamily: "monospace",
                fontSize: 11,
                color: t.text,
                "::placeholder": { color: t.lineNum } as React.CSSProperties,
              }}
            />
            {search && (
              <>
                <span
                  style={{
                    fontSize: 9,
                    color: t.accent,
                    fontFamily: "monospace",
                    fontWeight: 600,
                  }}
                >
                  {matchCount}
                </span>
                <button
                  onClick={() => setSearch("")}
                  style={{
                    background: "none",
                    border: "none",
                    color: t.muted,
                    cursor: "pointer",
                    fontSize: 12,
                    padding: 0,
                  }}
                >
                  ×
                </button>
              </>
            )}
          </div>
        )}

        {/* Copy */}
        <button
          onClick={copy}
          style={{
            ...toolBtn(t),
            borderColor: copied ? t.accent : t.border,
            color: copied ? t.accent : t.muted,
            background: copied ? t.accentDim : t.tab,
          }}
        >
          {copied ? "✓ Copied" : "⎘ Copy"}
        </button>
      </div>

      {/* ── Content ── */}
      <div
        style={{
          overflowX: "auto",
          maxHeight: 560,
          overflowY: "auto",
          scrollbarWidth: "thin",
          scrollbarColor: `${t.border} ${t.bg}`,
        }}
      >
        {mode === "tree" ? (
          <div style={{ minWidth: "100%", paddingBottom: 8, paddingTop: 4 }}>
            <TreeNode
              value={safeData}
              depth={0}
              isLast={true}
              theme={t}
              defaultExpandDepth={defaultExpandDepth}
              search={search}
              path="root"
              expandedPaths={expandedPaths}
              onToggle={togglePath}
              lineRef={lineRef}
            />
          </div>
        ) : (
          <div style={{ display: "flex" }}>
            {/* Line numbers */}
            <div
              style={{
                background: t.gutter,
                borderRight: `1px solid ${t.border}`,
                padding: "8px 0",
                flexShrink: 0,
                minWidth: 44,
                textAlign: "right",
              }}
            >
              {json.split("\n").map((_, i) => (
                <div
                  key={i}
                  style={{
                    fontFamily: "monospace",
                    fontSize: 10,
                    color: t.lineNum,
                    lineHeight: "20px",
                    paddingRight: 12,
                    userSelect: "none",
                  }}
                >
                  {i + 1}
                </div>
              ))}
            </div>
            {/* Code */}
            <pre
              style={{
                flex: 1,
                margin: 0,
                padding: "8px 14px",
                lineHeight: "20px",
                fontFamily: "'Fira Code', monospace",
                fontSize: 12,
                color: t.text,
                whiteSpace: "pre",
                overflowX: "auto",
              }}
              dangerouslySetInnerHTML={{ __html: rawHighlighted }}
            />
          </div>
        )}
      </div>

      {/* ── Footer ── */}
      <div
        style={{
          padding: "6px 16px",
          background: t.surface,
          borderTop: `1px solid ${t.border}`,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <div style={{ display: "flex", gap: 10 }}>
          {[
            { label: "keys", color: t.key },
            { label: "strings", color: t.str },
            { label: "numbers", color: t.num },
            { label: "booleans", color: t.bool },
            { label: "null", color: t.nil },
          ].map(({ label, color }) => (
            <span
              key={label}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                fontFamily: "monospace",
                fontSize: 9,
                color: t.muted,
              }}
            >
              <span
                style={{
                  width: 6,
                  height: 6,
                  borderRadius: 2,
                  background: color,
                  display: "inline-block",
                }}
              />
              {label}
            </span>
          ))}
        </div>
        <span
          style={{ fontFamily: "monospace", fontSize: 9, color: t.lineNum }}
        >
          {themeName}
        </span>
      </div>
    </div>
  );
}

function toolBtn(t: (typeof THEMES)[ViewerTheme]): React.CSSProperties {
  return {
    padding: "5px 10px",
    fontSize: 10,
    fontWeight: 600,
    letterSpacing: "0.06em",
    fontFamily: "monospace",
    cursor: "pointer",
    borderRadius: 6,
    border: `1px solid ${t.border}`,
    background: t.tab,
    color: t.muted,
    transition: "all 0.15s",
    flexShrink: 0,
    whiteSpace: "nowrap",
  };
}

// ─── Demo Payloads ────────────────────────────────────────────────────────────

const DEMO_PAYLOADS: Array<{
  label: string;
  tag: string;
  tagColor: string;
  title: string;
  subtitle: string;
  status: number;
  responseTime: number;
  theme: ViewerTheme;
  data: JsonValue;
}> = [
  {
    label: "Anchor Info",
    tag: "SEP-24",
    tagColor: "#a78bfa",
    title: "GET /sep24/info",
    subtitle: "testanchor.stellar.org",
    status: 200,
    responseTime: 143,
    theme: "ember",
    data: {
      deposit: {
        USDC: {
          enabled: true,
          authentication_required: true,
          fee_fixed: 0,
          fee_percent: 0,
          min_amount: 0.1,
          max_amount: 10000,
        },
        EURC: {
          enabled: true,
          authentication_required: true,
          fee_fixed: 0.5,
          fee_percent: 0.1,
          min_amount: 10,
          max_amount: 50000,
        },
      },
      withdraw: {
        USDC: {
          enabled: true,
          authentication_required: true,
          fee_fixed: 1,
          fee_percent: 0,
          min_amount: 1,
          max_amount: 10000,
          types: {
            sepa: {
              fields: {
                dest: { description: "IBAN" },
                dest_extra: { description: "BIC" },
              },
            },
          },
        },
      },
      fee: { enabled: false, authentication_required: false },
      features: { account_creation: true, claimable_balances: true },
    },
  },
  {
    label: "Transaction",
    tag: "SEP-6",
    tagColor: "#7dd3fc",
    title: "GET /transfer/transaction",
    subtitle: "id=a3d8f1c2-9b4e-4e7a",
    status: 200,
    responseTime: 87,
    theme: "arctic",
    data: {
      transaction: {
        id: "a3d8f1c2-9b4e-4e7a-b8d3-2c1f9e5a0b7d",
        kind: "deposit",
        status: "completed",
        status_eta: null,
        more_info_url:
          "https://testanchor.stellar.org/sep24/transaction?id=a3d8f1c2",
        amount_in: "100.00",
        amount_in_asset: "iso4217:USD",
        amount_out: "99.75",
        amount_out_asset:
          "stellar:USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
        amount_fee: "0.25",
        amount_fee_asset: "iso4217:USD",
        started_at: "2024-01-15T10:30:00.000Z",
        completed_at: "2024-01-15T11:05:32.000Z",
        updated_at: "2024-01-15T11:05:32.000Z",
        stellar_transaction_id:
          "4a7f8c3d2e1b9a6f5c0d3e8b1a4f7c2d5e8b1a4f7c2d9e6b3a0f5c8d1e4b7a",
        external_transaction_id: "ACH-2024011500123",
        message: null,
        refunds: null,
        claimable_balance_id: null,
      },
    },
  },
  {
    label: "Error Payload",
    tag: "400",
    tagColor: "#fca5a5",
    title: "POST /sep24/transactions/deposit/interactive",
    subtitle: "Missing required parameter",
    status: 400,
    responseTime: 22,
    theme: "forest",
    data: {
      error: "Missing required field",
      details: {
        field: "asset_code",
        message:
          "The 'asset_code' parameter is required to initiate a deposit.",
        received: null,
        allowed_values: ["USDC", "EURC", "XLM"],
        docs: "https://stellar.org/protocol/sep-24#fields",
      },
      request_id: "req_9f2c1a4e8b3d",
      timestamp: "2024-01-15T14:22:11.000Z",
    },
  },
  {
    label: "KYC Status",
    tag: "SEP-12",
    tagColor: "#f97316",
    title: "GET /kyc/customer",
    subtitle: "account=G...QT3V",
    status: 200,
    responseTime: 205,
    theme: "ember",
    data: {
      id: "c7e9b2f1-4a8d-4b3c-9e1f-2a5d8c3b7e0a",
      provided_fields: {
        first_name: {
          description: "First name",
          type: "string",
          status: "accepted",
          error: null,
        },
        last_name: {
          description: "Last name",
          type: "string",
          status: "accepted",
          error: null,
        },
        email_address: {
          description: "Email address",
          type: "string",
          status: "accepted",
          error: null,
        },
        date_of_birth: {
          description: "Date of birth",
          type: "date",
          status: "accepted",
          error: null,
        },
        id_type: {
          description: "Government ID type",
          type: "string",
          status: "processing",
          error: null,
        },
        id_number: {
          description: "Government ID number",
          type: "string",
          status: "processing",
          error: null,
        },
      },
      status: "NEEDS_INFO",
      message:
        "Your government ID is under review. This usually takes 1–3 business days.",
      fields: {
        address: {
          description: "Full street address",
          type: "string",
          optional: false,
        },
        country_code: {
          description: "Country of residence",
          type: "string",
          optional: false,
        },
      },
    },
  },
];

// ─── Demo Page ────────────────────────────────────────────────────────────────

export default function JsonViewerDemo() {
  const [active, setActive] = useState(0);
  const payload = DEMO_PAYLOADS[active];

  return (
    <div
      style={{
        minHeight: "100vh",
        background:
          "linear-gradient(135deg, #0e0c0a 0%, #12100e 50%, #0a0e12 100%)",
        padding: "40px 24px",
        fontFamily: "'Fira Code', monospace",
      }}
    >
      <style>{`
        @import url('https://fonts.googleapis.com/css2?family=Fira+Code:wght@400;500;600&display=swap');
        * { box-sizing: border-box; margin: 0; padding: 0; }
        ::-webkit-scrollbar { width: 6px; height: 6px; }
        ::-webkit-scrollbar-track { background: transparent; }
        ::-webkit-scrollbar-thumb { background: #2d2720; border-radius: 3px; }
        ::placeholder { color: #3d3530 !important; }
      `}</style>

      <div style={{ maxWidth: 900, margin: "0 auto" }}>
        {/* Header */}
        <div
          style={{
            marginBottom: 36,
            display: "flex",
            alignItems: "flex-end",
            justifyContent: "space-between",
          }}
        >
          <div>
            <div
              style={{
                fontSize: 9,
                letterSpacing: "0.22em",
                color: "#57534e",
                textTransform: "uppercase",
                marginBottom: 8,
              }}
            >
              Component · JSON Viewer
            </div>
            <h1
              style={{
                fontSize: 28,
                fontWeight: 700,
                color: "#e7e5e0",
                letterSpacing: "-0.03em",
                lineHeight: 1.2,
              }}
            >
              Response Viewer
            </h1>
            <p
              style={{
                fontSize: 11,
                color: "#78716c",
                marginTop: 6,
                lineHeight: 1.6,
              }}
            >
              Collapsible syntax-highlighted JSON for anchor responses, errors
              &amp; transaction data.
            </p>
          </div>

          {/* Theme dot indicators */}
          <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
            <span
              style={{
                fontSize: 9,
                color: "#57534e",
                marginRight: 4,
                letterSpacing: "0.1em",
              }}
            >
              THEMES
            </span>
            {(
              [
                ["ember", "#f97316"],
                ["arctic", "#58a6ff"],
                ["forest", "#4ade80"],
              ] as [ViewerTheme, string][]
            ).map(([th, c]) => (
              <div
                key={th}
                style={{
                  width: 10,
                  height: 10,
                  borderRadius: "50%",
                  background: c,
                  opacity: 0.7,
                  boxShadow: `0 0 8px ${c}60`,
                }}
                title={th}
              />
            ))}
          </div>
        </div>

        {/* Payload switcher */}
        <div
          style={{
            display: "flex",
            gap: 8,
            marginBottom: 24,
            flexWrap: "wrap",
          }}
        >
          {DEMO_PAYLOADS.map((p, i) => (
            <button
              key={i}
              onClick={() => setActive(i)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                padding: "8px 14px",
                borderRadius: 9,
                border: `1px solid ${active === i ? p.tagColor + "60" : "#2d2720"}`,
                background: active === i ? `${p.tagColor}12` : "#1c1916",
                cursor: "pointer",
                fontFamily: "monospace",
                transition: "all 0.18s",
              }}
            >
              <span
                style={{
                  fontSize: 9,
                  fontWeight: 700,
                  padding: "2px 6px",
                  borderRadius: 4,
                  background: `${p.tagColor}25`,
                  color: p.tagColor,
                }}
              >
                {p.tag}
              </span>
              <span
                style={{
                  fontSize: 11,
                  color: active === i ? "#e7e5e0" : "#78716c",
                }}
              >
                {p.label}
              </span>
            </button>
          ))}
        </div>

        {/* Viewer */}
        <JsonViewer
          key={active}
          data={payload.data}
          title={payload.title}
          subtitle={payload.subtitle}
          status={payload.status}
          responseTime={payload.responseTime}
          theme={payload.theme}
          defaultMode="tree"
          defaultExpandDepth={2}
          searchable
        />

        {/* Usage snippet */}
        <div
          style={{
            marginTop: 24,
            background: "#1c1916",
            borderRadius: 12,
            border: "1px solid #2d2720",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              padding: "10px 16px",
              background: "#141210",
              borderBottom: "1px solid #2d2720",
              fontSize: 9,
              letterSpacing: "0.18em",
              color: "#57534e",
              textTransform: "uppercase",
            }}
          >
            Usage
          </div>
          <pre
            style={{
              padding: "16px 18px",
              fontFamily: "'Fira Code', monospace",
              fontSize: 11,
              color: "#78716c",
              lineHeight: 1.8,
              overflowX: "auto",
            }}
          >{`<JsonViewer
  data={responseJson}
  title="GET /sep24/info"
  subtitle="testanchor.stellar.org"
  status={200}
  responseTime={143}
  theme="ember"          // "ember" | "arctic" | "forest"
  defaultMode="tree"     // "tree" | "raw"
  defaultExpandDepth={2}
  searchable
/>`}</pre>
        </div>
      </div>
    </div>
  );
}
