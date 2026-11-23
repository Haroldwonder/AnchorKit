/**
 * @anchorkit/ui-components — public package entrypoint
 *
 * Re-exports every public symbol from:
 *   - components/  (React UI components)
 *   - hooks/       (React hooks)
 *   - src/         (SDK config)
 *
 * This file must stay at ui/index.ts so that `tsc` emits dist/index.js,
 * matching the "main" / "types" fields declared in package.json.
 */

// ── Components ──────────────────────────────────────────────────────────────
export {
  ApiRequestPanel,
  AnchorHealthBadge,
  AnchorSelector,
  TransactionTimeline,
  AnchorCapabilityCard,
  AnchorErrorBoundary,
  withAnchorErrorBoundary,
  AnchorPlayground,
  JsonViewer,
  Sep10AuthFlow,
  PrecisionFintech,
  SkeletonLoader,
  AssetListSkeleton,
  FeeTableSkeleton,
  LimitsSkeleton,
  EmptyState,
} from './components';

export type {
  ApiRequestPanelProps,
  AnchorHealthBadgeProps,
  AnchorSelectorProps,
  AnchorOption,
  TransactionTimelineProps,
  TxEvent,
  TxStatus,
  TxType,
  AnchorCapabilityCardProps,
  KYCLevel,
  OperationType,
  AssetFee,
  AssetLimits,
  KYCField,
  KYCRequirement,
  SupportedAsset,
  ServiceHealth,
  AnchorServices,
  AnchorHealthStatus,
  AnchorErrorBoundaryProps,
  AnchorKitError,
  JsonViewerProps,
  ViewerTheme,
  ViewerMode,
  SkeletonLoaderProps,
  EmptyStateProps,
} from './components';

// ── Hooks ────────────────────────────────────────────────────────────────────
export {
  useCopyToClipboard,
  formatJsonForCopy,
  generateCurlCommand,
  generateInstallCommand,
  useTheme,
  useRateLimitStatus,
  clearRateLimitCache,
  getRateLimitStatus,
  ContractError,
  useSep10Auth,
  useAnchorHealth,
  isValidAttestor,
  useTransactionStatus,
  TransactionStateTracker,
  isTerminalStatus,
  useAnchorCapabilities,
  clearCapabilitiesCache,
  SERVICE_DEPOSITS,
  SERVICE_WITHDRAWALS,
  SERVICE_QUOTES,
  SERVICE_KYC,
} from './hooks';

export type {
  CopyToClipboardOptions,
  CopyToClipboardResult,
  RateLimitStatus,
  UseRateLimitStatusResult,
  UseRateLimitStatusOptions,
  RateLimitStatusRaw,
  Sep10AuthAdapters,
  UseSep10AuthResult,
  GetHealthScoreFn,
  UseAnchorHealthResult,
  UseTransactionStatusOptions,
  UseTransactionStatusResult,
  FetchStatusFn,
  TransactionTransition,
  TransactionSnapshot,
  AnchorServicesResult,
  FetchCapabilitiesFn,
  UseAnchorCapabilitiesOptions,
  UseAnchorCapabilitiesResult,
} from './hooks';

// ── SDK Config ───────────────────────────────────────────────────────────────
export { config } from './src';
export type { Config } from './src';
