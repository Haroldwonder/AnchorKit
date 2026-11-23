import React, { Component, ReactNode } from "react";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface AnchorKitError {
  message: string;
  code?: string | number;
  context?: Record<string, unknown>;
}

export interface AnchorErrorBoundaryProps {
  /** Component(s) to render when no error is present */
  children: ReactNode;
  /** Custom fallback UI. Receives the caught error + reset handler. */
  fallback?: (error: AnchorKitError, reset: () => void) => ReactNode;
  /** Called whenever an error is caught — use for logging / analytics */
  onError?: (error: AnchorKitError, errorInfo: React.ErrorInfo) => void;
  /** Optional label shown in the default fallback (e.g. "Anchor Feed", "Price Widget") */
  componentLabel?: string;
  /** When any value in this array changes, the error state is automatically reset */
  resetKeys?: unknown[];
}

interface State {
  hasError: boolean;
  error: AnchorKitError | null;
  prevResetKeys: unknown[] | undefined;
  retryKey: number;
}

// ─── Normalise any thrown value into an AnchorKitError ────────────────────────

function normaliseError(raw: unknown): AnchorKitError {
  if (raw instanceof Error) {
    return {
      message: raw.message,
      code: (raw as Error & { code?: string | number }).code,
    };
  }
  if (typeof raw === "object" && raw !== null && "message" in raw) {
    return raw as AnchorKitError;
  }
  return { message: String(raw) };
}

// ─── Default Fallback UI ──────────────────────────────────────────────────────

function DefaultFallback({
  error,
  reset,
  label,
}: {
  error: AnchorKitError;
  reset: () => void;
  label?: string;
}) {
  return (
    <div
      role="alert"
      aria-live="assertive"
      style={{
        fontFamily: "'DM Mono', 'Courier New', monospace",
        background: "#0a0a0a",
        border: "1px solid #1f1f1f",
        borderLeft: "3px solid #ff4d6d",
        borderRadius: 2,
        padding: "20px 24px",
        display: "flex",
        flexDirection: "column",
        gap: 12,
        minWidth: 280,
        maxWidth: 480,
      }}
    >
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <span
          aria-hidden="true"
          style={{
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            width: 28,
            height: 28,
            border: "1px solid #ff4d6d",
            color: "#ff4d6d",
            fontSize: 13,
            flexShrink: 0,
          }}
        >
          ✕
        </span>
        <div>
          <div
            style={{
              fontSize: 9,
              letterSpacing: "0.2em",
              color: "#ff4d6d",
              marginBottom: 2,
            }}
          >
            ANCHOR SDK — COMPONENT FAILURE
          </div>
          <div
            style={{
              fontSize: 13,
              color: "#e0e0e0",
              letterSpacing: "-0.01em",
            }}
          >
            {label ?? "SDK Component"} unavailable
          </div>
        </div>
      </div>

      {/* Divider */}
      <div style={{ height: 1, background: "#1a1a1a" }} />

      {/* Error detail */}
      <div
        style={{
          background: "#050505",
          border: "1px solid #161616",
          padding: "10px 14px",
          fontSize: 11,
          color: "#666",
          letterSpacing: "0.02em",
          lineHeight: 1.6,
          wordBreak: "break-word",
        }}
      >
        <span style={{ color: "#ff4d6d", marginRight: 6 }}>
          {error.code ? `[${error.code}]` : "[ERR]"}
        </span>
        {error.message}
      </div>

      {/* Context dump (optional) */}
      {error.context && Object.keys(error.context).length > 0 && (
        <details style={{ fontSize: 10, color: "#444", cursor: "pointer" }}>
          <summary style={{ letterSpacing: "0.1em", outline: "none" }}>
            CONTEXT
          </summary>
          <pre
            style={{
              marginTop: 8,
              padding: "8px 12px",
              background: "#050505",
              border: "1px solid #111",
              color: "#555",
              fontSize: 10,
              lineHeight: 1.7,
              overflowX: "auto",
            }}
          >
            {JSON.stringify(error.context, null, 2)}
          </pre>
        </details>
      )}

      {/* Actions */}
      <div style={{ display: "flex", gap: 8, marginTop: 4 }}>
        <button
          onClick={reset}
          aria-label="Try Again"
          style={{
            flex: 1,
            padding: "9px 0",
            background: "transparent",
            border: "1px solid rgba(0,255,178,0.3)",
            color: "#00FFB2",
            fontSize: 10,
            letterSpacing: "0.12em",
            cursor: "pointer",
            fontFamily: "DM Mono, monospace",
            transition: "all 0.15s",
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLButtonElement).style.background =
              "rgba(0,255,178,0.08)";
            (e.currentTarget as HTMLButtonElement).style.borderColor =
              "#00FFB2";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLButtonElement).style.background =
              "transparent";
            (e.currentTarget as HTMLButtonElement).style.borderColor =
              "rgba(0,255,178,0.3)";
          }}
        >
          RETRY
        </button>
        <button
          onClick={() => window.location.reload()}
          style={{
            flex: 1,
            padding: "9px 0",
            background: "transparent",
            border: "1px solid #1f1f1f",
            color: "#555",
            fontSize: 10,
            letterSpacing: "0.12em",
            cursor: "pointer",
            fontFamily: "DM Mono, monospace",
            transition: "all 0.15s",
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLButtonElement).style.color = "#888";
            (e.currentTarget as HTMLButtonElement).style.borderColor = "#333";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLButtonElement).style.color = "#555";
            (e.currentTarget as HTMLButtonElement).style.borderColor =
              "#1f1f1f";
          }}
        >
          RELOAD
        </button>
      </div>
    </div>
  );
}

// ─── Error Boundary Class ─────────────────────────────────────────────────────

export class AnchorErrorBoundary extends Component<
  AnchorErrorBoundaryProps,
  State
> {
  constructor(props: AnchorErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null, prevResetKeys: props.resetKeys, retryKey: 0 };
    this.reset = this.reset.bind(this);
  }

  static getDerivedStateFromProps(
    props: AnchorErrorBoundaryProps,
    state: State,
  ): Partial<State> | null {
    // Check if resetKeys has actually changed
    const resetKeysChanged =
      props.resetKeys !== state.prevResetKeys &&
      !(
        props.resetKeys === undefined &&
        state.prevResetKeys === undefined
      ) &&
      !(
        Array.isArray(props.resetKeys) &&
        Array.isArray(state.prevResetKeys) &&
        props.resetKeys.length === state.prevResetKeys.length &&
        props.resetKeys.every((key, i) => Object.is(key, state.prevResetKeys![i]))
      );

    // If there's an error and resetKeys changed, reset the error state
    if (state.hasError && resetKeysChanged && props.resetKeys !== undefined) {
      return { hasError: false, error: null, prevResetKeys: props.resetKeys, retryKey: state.retryKey + 1 };
    }

    // Always update prevResetKeys to track changes
    if (props.resetKeys !== state.prevResetKeys) {
      return { prevResetKeys: props.resetKeys };
    }

    return null;
  }

  static getDerivedStateFromError(raw: unknown): Partial<State> {
    return { hasError: true, error: normaliseError(raw) };
  }

  componentDidCatch(raw: unknown, errorInfo: React.ErrorInfo): void {
    const error = normaliseError(raw);
    this.props.onError?.(error, errorInfo);
  }

  reset(): void {
    this.setState((s) => ({ hasError: false, error: null, retryKey: s.retryKey + 1 }));
  }

  render(): ReactNode {
    const { hasError, error, retryKey } = this.state;
    const { children, fallback, componentLabel } = this.props;

    if (!hasError || !error) return <React.Fragment key={retryKey}>{children}</React.Fragment>;

    if (fallback) return fallback(error, this.reset);

    return (
      <DefaultFallback
        error={error}
        reset={this.reset}
        label={componentLabel}
      />
    );
  }
}

// ─── Convenience HOC ─────────────────────────────────────────────────────────

/**
 * Wraps any component in an AnchorErrorBoundary.
 *
 * @example
 * const SafeAnchorFeed = withAnchorErrorBoundary(AnchorFeed, { componentLabel: "Anchor Feed" });
 */
export function withAnchorErrorBoundary<P extends object>(
  WrappedComponent: React.ComponentType<P>,
  boundaryProps?: Omit<AnchorErrorBoundaryProps, "children">,
): React.FC<P> {
  const displayName =
    WrappedComponent.displayName ?? WrappedComponent.name ?? "Component";

  const WrappedWithBoundary: React.FC<P> = (props) => (
    <AnchorErrorBoundary componentLabel={displayName} {...boundaryProps}>
      <WrappedComponent {...props} />
    </AnchorErrorBoundary>
  );

  WrappedWithBoundary.displayName = `withAnchorErrorBoundary(${displayName})`;
  return WrappedWithBoundary;
}

// ─── Exports ──────────────────────────────────────────────────────────────────

export default AnchorErrorBoundary;
