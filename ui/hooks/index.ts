/**
 * AnchorKit UI Hooks
 * 
 * Reusable React hooks for common UI patterns
 */

export {
  useCopyToClipboard,
  formatJsonForCopy,
  generateCurlCommand,
  generateInstallCommand,
  type CopyToClipboardOptions,
  type CopyToClipboardResult,
} from './useCopyToClipboard';
export { useTheme } from './useTheme';
export {
  useRateLimitStatus,
  clearRateLimitCache,
  type RateLimitStatus,
  type UseRateLimitStatusResult,
  type UseRateLimitStatusOptions,
} from './useRateLimitStatus';
export {
  getRateLimitStatus,
  setRateLimitClient,
  resetRateLimitClient,
  type RateLimitStatusFetcher,
  type RateLimitStatusRaw,
  ContractError,
} from './contractClient';
export { useSep10Auth } from './useSep10Auth';
export type { Sep10AuthAdapters, UseSep10AuthResult } from './useSep10Auth';
export { useAnchorHealth, isValidAttestor } from './useAnchorHealth';
export type { GetHealthScoreFn, UseAnchorHealthResult } from './useAnchorHealth';
export { useTransactionStatus } from './useTransactionStatus';
export type {
  UseTransactionStatusOptions,
  UseTransactionStatusResult,
  FetchStatusFn,
  TransactionTransition,
  TransactionSnapshot,
} from './useTransactionStatus';
export { TransactionStateTracker, isTerminalStatus } from './TransactionStateTracker';
export { useAnchorCapabilities, clearCapabilitiesCache } from './useAnchorCapabilities';
export type {
  AnchorServicesResult,
  FetchCapabilitiesFn,
  UseAnchorCapabilitiesOptions,
  UseAnchorCapabilitiesResult,
} from './useAnchorCapabilities';
export {
  SERVICE_DEPOSITS,
  SERVICE_WITHDRAWALS,
  SERVICE_QUOTES,
  SERVICE_KYC,
} from './useAnchorCapabilities';
