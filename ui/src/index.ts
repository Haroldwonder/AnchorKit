// NOTE: Do not load or validate server-only config at import time.
// `@anchorkit/ui-components` is a browser-facing React component library, and
// `ui/src/config` validates backend-only secrets (DATABASE_URL, ADMIN_SECRET_KEY,
// STELLAR_RPC_URL, ...) via `process.env`, calling `process.exit(1)` on failure.
// Running that here would crash any browser bundle that imports this entry.
// Backend consumers should import the loader directly from './config'.
export { loadConfig } from './config';
export type { Config } from './config';

// Internationalization utilities
export {
  I18nProvider,
  useI18n,
  useTranslate,
  type Locale,
  type MessageKey,
  type MessageCatalog,
  type LocaleMessages,
  type I18nContextValue,
  type I18nProviderProps,
  defaultMessages,
  localeMessages,
} from './i18n';

// Intl formatting utilities
export {
  formatAmount,
  formatRawAmount,
  formatDateTime,
  formatDate,
  formatTime,
  formatRelativeTime,
  formatDuration,
  getAssetInfo,
  useFormatAmount,
  useFormatDateTime,
  useFormatRelativeTime,
  type AssetInfo,
  type FormatAmountOptions,
  type FormatDateTimeOptions,
  type FormatRelativeTimeOptions,
} from './intl';

// SEP-10 Challenge verification
export {
  verifyChallenge,
  createChallengeVerifier,
  type VerifyChallengeOptions,
  type VerifyChallengeResult,
  type Sep10ChallengeVerificationConfig,
} from './sep10Challenge';
