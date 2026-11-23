// NOTE: Do not load or validate server-only config at import time.
// `@anchorkit/ui-components` is a browser-facing React component library, and
// `ui/src/config` validates backend-only secrets (DATABASE_URL, ADMIN_SECRET_KEY,
// STELLAR_RPC_URL, ...) via `process.env`, calling `process.exit(1)` on failure.
// Running that here would crash any browser bundle that imports this entry.
// Backend consumers should import the loader directly from './config'.
export { loadConfig } from './config';
export type { Config } from './config';
