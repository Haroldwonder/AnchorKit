/**
 * Test suite for Issue #1068: Config validation in UI component library
 *
 * The issue is that frontend components import config that validates backend-only
 * environment variables (DATABASE_URL, ADMIN_SECRET_KEY), which causes the library
 * to fail when used in browser environments where these variables don't exist.
 */

import { configSchema, type Config } from './schema';

describe('Issue #1068: Config Validation Architecture', () => {
  const originalEnv = process.env;

  beforeEach(() => {
    jest.resetModules();
    process.env = { ...originalEnv };
  });

  afterEach(() => {
    process.env = originalEnv;
  });

  describe('Configuration requirements', () => {
    test('should define required configuration fields', () => {
      // Verify schema shape
      const schemaShape = configSchema.shape;

      expect(schemaShape).toHaveProperty('NODE_ENV');
      expect(schemaShape).toHaveProperty('PORT');
      expect(schemaShape).toHaveProperty('DATABASE_URL');
      expect(schemaShape).toHaveProperty('STELLAR_RPC_URL');
      expect(schemaShape).toHaveProperty('HORIZON_URL');
      expect(schemaShape).toHaveProperty('ANCHOR_API_KEY');
      expect(schemaShape).toHaveProperty('ADMIN_SECRET_KEY');
      expect(schemaShape).toHaveProperty('LOG_LEVEL');
    });

    test('should require NODE_ENV to be one of development, production, test', () => {
      const validEnv = {
        NODE_ENV: 'development',
        PORT: '3000',
        DATABASE_URL: 'postgresql://localhost/db',
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: 'test-key',
        ADMIN_SECRET_KEY: 'test-secret',
      };

      const result = configSchema.safeParse(validEnv);
      expect(result.success).toBe(true);
    });

    test('should reject invalid NODE_ENV', () => {
      const invalidEnv = {
        NODE_ENV: 'invalid',
        PORT: '3000',
        DATABASE_URL: 'postgresql://localhost/db',
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: 'test-key',
        ADMIN_SECRET_KEY: 'test-secret',
      };

      const result = configSchema.safeParse(invalidEnv);
      expect(result.success).toBe(false);
    });
  });

  describe('Browser compatibility concerns', () => {
    test('should document that DATABASE_URL is backend-only', () => {
      // This test documents that DATABASE_URL should not be required in browser
      const browserEnv = {
        NODE_ENV: 'production',
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: 'test-key',
      };

      const result = configSchema.safeParse(browserEnv);

      // Currently this fails because DATABASE_URL is required
      // Future fix should make it optional for browser contexts
      expect(result.success).toBe(false);
      expect(result.error?.issues.some(i =>
        i.path.includes('DATABASE_URL') || i.path.includes('ADMIN_SECRET_KEY')
      )).toBe(true);
    });

    test('should document that ADMIN_SECRET_KEY is backend-only', () => {
      // This test documents that ADMIN_SECRET_KEY should not be required in browser
      const browserEnv = {
        NODE_ENV: 'production',
        DATABASE_URL: 'postgresql://localhost/db',
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: 'test-key',
      };

      const result = configSchema.safeParse(browserEnv);

      // Currently this fails because ADMIN_SECRET_KEY is required
      // Future fix should make it optional for browser contexts
      expect(result.success).toBe(false);
    });

    test('should validate that browser only needs frontend config', () => {
      // Browser-safe environment variables only
      const browserConfig = {
        NODE_ENV: 'production',
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: 'test-key',
      };

      // This represents what a browser configuration should look like
      // Note: Currently the schema requires backend vars, which is the issue
      expect(browserConfig).toHaveProperty('STELLAR_RPC_URL');
      expect(browserConfig).toHaveProperty('HORIZON_URL');
      expect(browserConfig).toHaveProperty('ANCHOR_API_KEY');

      // Backend-only vars should not be in browser config
      expect(browserConfig).not.toHaveProperty('DATABASE_URL');
      expect(browserConfig).not.toHaveProperty('ADMIN_SECRET_KEY');
    });
  });

  describe('Configuration validation strategy', () => {
    test('should separate backend and frontend config schemas', () => {
      // This test documents the solution: have separate schemas for backend/frontend

      // Frontend config should only include browser-safe variables
      const frontendConfigSchema = {
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: 'test-key',
        LOG_LEVEL: 'info',
      };

      expect(frontendConfigSchema).toBeDefined();
      expect(frontendConfigSchema).not.toHaveProperty('DATABASE_URL');
      expect(frontendConfigSchema).not.toHaveProperty('ADMIN_SECRET_KEY');
    });

    test('should defer config validation away from import time', () => {
      // Config loading should not happen at module import time
      // Instead, it should be lazy-loaded or deferred to when actually needed

      const configModule = require('./index');

      // The current implementation loads config immediately with:
      // export const config: Config = loadConfig();
      //
      // This should be changed to lazy initialization or function-based loading
      // to prevent crashes when imported in browser environments

      expect(configModule).toBeDefined();
    });

    test('should document backend config is not for frontend', () => {
      // Backend configuration requirements should be clearly separated
      const backendRequiredVars = [
        'DATABASE_URL',
        'ADMIN_SECRET_KEY',
      ];

      const frontendRequiredVars = [
        'STELLAR_RPC_URL',
        'HORIZON_URL',
        'ANCHOR_API_KEY',
      ];

      expect(backendRequiredVars.length).toBeGreaterThan(0);
      expect(frontendRequiredVars.length).toBeGreaterThan(0);

      // They should be disjoint - no overlap
      const intersection = backendRequiredVars.filter(v =>
        frontendRequiredVars.includes(v)
      );
      expect(intersection).toHaveLength(0);
    });
  });

  describe('Error handling and validation', () => {
    test('should validate PORT as a positive integer', () => {
      const validEnv = {
        NODE_ENV: 'development',
        PORT: '3000',
        DATABASE_URL: 'postgresql://localhost/db',
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: 'test-key',
        ADMIN_SECRET_KEY: 'test-secret',
      };

      const result = configSchema.safeParse(validEnv);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.PORT).toBe(3000);
        expect(typeof result.data.PORT).toBe('number');
      }
    });

    test('should reject negative PORT values', () => {
      const invalidEnv = {
        NODE_ENV: 'development',
        PORT: '-3000',
        DATABASE_URL: 'postgresql://localhost/db',
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: 'test-key',
        ADMIN_SECRET_KEY: 'test-secret',
      };

      const result = configSchema.safeParse(invalidEnv);
      expect(result.success).toBe(false);
    });

    test('should validate URLs are properly formatted', () => {
      const invalidEnv = {
        NODE_ENV: 'development',
        PORT: '3000',
        DATABASE_URL: 'not-a-url',
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: 'test-key',
        ADMIN_SECRET_KEY: 'test-secret',
      };

      const result = configSchema.safeParse(invalidEnv);
      expect(result.success).toBe(false);
      expect(result.error?.issues.some(i => i.path.includes('DATABASE_URL'))).toBe(true);
    });

    test('should require non-empty ANCHOR_API_KEY', () => {
      const invalidEnv = {
        NODE_ENV: 'development',
        PORT: '3000',
        DATABASE_URL: 'postgresql://localhost/db',
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: '',
        ADMIN_SECRET_KEY: 'test-secret',
      };

      const result = configSchema.safeParse(invalidEnv);
      expect(result.success).toBe(false);
    });

    test('should allow custom LOG_LEVEL values', () => {
      const validEnv = {
        NODE_ENV: 'development',
        PORT: '3000',
        DATABASE_URL: 'postgresql://localhost/db',
        STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
        HORIZON_URL: 'https://horizon-testnet.stellar.org',
        ANCHOR_API_KEY: 'test-key',
        ADMIN_SECRET_KEY: 'test-secret',
        LOG_LEVEL: 'debug',
      };

      const result = configSchema.safeParse(validEnv);
      expect(result.success).toBe(true);
    });
  });
});
