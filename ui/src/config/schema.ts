import { z } from 'zod';

/**
 * Server-only configuration schema.
 *
 * This schema validates backend secrets (DATABASE_URL, ADMIN_SECRET_KEY,
 * STELLAR_RPC_URL, ...) and must NOT be evaluated at import time by the
 * browser-facing entry point (`ui/src/index.ts`). Importing this module is
 * safe: it only declares the schema and types, it never reads `process.env`
 * nor calls `process.exit`. Consumers that need validation must call
 * `loadConfig()` explicitly from a server/Node context.
 */
export const configSchema = z.object({
  NODE_ENV: z.enum(['development', 'production', 'test']),
  PORT: z.coerce.number().int().positive(),
  DATABASE_URL: z.string().url(),
  STELLAR_RPC_URL: z.string().url(),
  HORIZON_URL: z.string().url(),
  ANCHOR_API_KEY: z.string().min(1),
  ADMIN_SECRET_KEY: z.string().min(1),
  LOG_LEVEL: z.enum(['debug', 'info', 'warn', 'error']).default('info'),
});

export type Config = z.infer<typeof configSchema>;

/**
 * Server-side config loader.
 *
 * Kept intact for backend consumers. It is intentionally NOT invoked at
 * module-import time so that importing this file (directly or transitively
 * from the browser bundle) never touches `process.env` or `process.exit`.
 * Callers must invoke `loadConfig()` explicitly in a Node/server runtime.
 */
export function loadConfig(env: NodeJS.ProcessEnv = process.env): Config {
  const result = configSchema.safeParse(env);
  if (!result.success) {
    // eslint-disable-next-line no-console
    console.error('Invalid server configuration:', result.error.flatten().fieldErrors);
    process.exit(1);
  }
  return result.data;
}
