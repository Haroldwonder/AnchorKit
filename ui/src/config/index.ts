import { configSchema, type Config } from './schema';

/**
 * Load and validate the server-side configuration from `process.env`.
 *
 * This is intentionally lazy: it must only be invoked from server-side
 * entry points. Importing this module from a browser bundle must not
 * touch `process.env` or `process.exit`.
 */
export function loadConfig(): Config {
  const result = configSchema.safeParse(process.env);

  if (!result.success) {
    const errors = result.error.issues
      .map((issue) => `  • ${issue.path.join('.')}: ${issue.message}`)
      .join('\n');

    console.error(
      '\n' +
      '╔══════════════════════════════════════════════════╗\n' +
      '║        INVALID ENVIRONMENT CONFIGURATION         ║\n' +
      '╚══════════════════════════════════════════════════╝\n' +
      `${errors}\n`
    );

    process.exit(1);
  }

  // result.success is true here; data is guaranteed to be defined.
  return result.data as Config;
}

export type { Config };
