/**
 * SEP-10 Challenge Transaction Verification
 *
 * Before a client signs a SEP-10 challenge it should verify:
 * - source account = server signing key from stellar.toml
 * - sequence = 0
 * - first op is manage_data named <home_domain> auth
 * - web_auth_domain op matches the auth endpoint host
 * - time bounds are valid
 * - no unexpected operations
 *
 * Without this a malicious server can get the client to sign a real transaction.
 */

import { xdr, StrKey, Networks } from '@stellar/stellar-sdk';

export interface VerifyChallengeOptions {
  /** The server's signing public key from stellar.toml (ED25519) */
  serverSigningKey: string;
  /** The anchor's home domain (e.g., "anchor.example.com") */
  homeDomain: string;
  /** The auth endpoint host (e.g., "anchor.example.com") */
  webAuthDomain: string;
  /** Optional: maximum age of timebounds in seconds (default: 300) */
  maxTimeboundsAge?: number;
  /** Optional: allow sequence number > 0 for testing (default: false) */
  allowNonZeroSequence?: boolean;
}

export interface VerifyChallengeResult {
  valid: boolean;
  error?: string;
  /** Parsed transaction details for debugging */
  details?: {
    sourceAccount: string;
    sequence: string;
    operations: Array<{
      type: string;
      name?: string;
      authRequired?: boolean;
    }>;
    timeBounds?: { minTime: string; maxTime: string };
  };
}

/**
 * Verify a SEP-10 challenge transaction XDR before signing.
 *
 * @param challengeXdr - Base64-encoded challenge transaction XDR
 * @param options - Verification options
 * @returns Verification result
 */
export function verifyChallenge(
  challengeXdr: string,
  options: VerifyChallengeOptions,
): VerifyChallengeResult {
  const {
    serverSigningKey,
    homeDomain,
    webAuthDomain,
    maxTimeboundsAge = 300,
    allowNonZeroSequence = false,
  } = options;

  try {
    // Parse the transaction envelope
    let envelope: xdr.TransactionEnvelope;
    try {
      envelope = xdr.TransactionEnvelope.fromXDR(challengeXdr, 'base64');
    } catch (e) {
      return {
        valid: false,
        error: `Failed to parse challenge XDR: ${(e as Error).message}`,
      };
    }

    const tx = envelope.v1().tx();
    const sourceAccount = tx.sourceAccountEd25519();
    const sequence = tx.seqNum();
    const operations = tx.operations();
    const timeBounds = tx.timeBounds();

    const sourceAccountStr = StrKey.encodeEd25519PublicKey(sourceAccount);
    const sequenceStr = sequence.toString();

    // 1. Verify source account matches server signing key
    if (sourceAccountStr !== serverSigningKey) {
      return {
        valid: false,
        error: `Source account mismatch: expected ${serverSigningKey}, got ${sourceAccountStr}`,
        details: {
          sourceAccount: sourceAccountStr,
          sequence: sequenceStr,
          operations: operations.map((op) => ({
            type: op.body().switch().name,
            name: op.body().manageDataOp()?.dataName?.toString(),
            authRequired: op.body().manageDataOp() !== undefined,
          })),
          timeBounds: timeBounds
            ? { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() }
            : undefined,
        },
      };
    }

    // 2. Verify sequence number is 0
    if (!allowNonZeroSequence && sequence.toString() !== '0') {
      return {
        valid: false,
        error: `Invalid sequence number: expected 0, got ${sequenceStr}`,
        details: {
          sourceAccount: sourceAccountStr,
          sequence: sequenceStr,
          operations: operations.map((op) => ({
            type: op.body().switch().name,
            name: op.body().manageDataOp()?.dataName?.toString(),
            authRequired: op.body().manageDataOp() !== undefined,
          })),
          timeBounds: timeBounds
            ? { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() }
            : undefined,
        },
      };
    }

    // 3. Verify there's exactly one operation (or at least first is manage_data)
    if (operations.length === 0) {
      return {
        valid: false,
        error: 'Challenge transaction has no operations',
        details: {
          sourceAccount: sourceAccountStr,
          sequence: sequenceStr,
          operations: [],
          timeBounds: timeBounds
            ? { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() }
            : undefined,
        },
      };
    }

    // 4. Verify first operation is manage_data with name = "<home_domain> auth"
    const firstOp = operations[0];
    const firstOpBody = firstOp.body();

    if (firstOpBody.switch().name !== 'manageData') {
      return {
        valid: false,
        error: `First operation must be manage_data, got ${firstOpBody.switch().name}`,
        details: {
          sourceAccount: sourceAccountStr,
          sequence: sequenceStr,
          operations: operations.map((op) => ({
            type: op.body().switch().name,
            name: op.body().manageDataOp()?.dataName?.toString(),
            authRequired: op.body().manageDataOp() !== undefined,
          })),
          timeBounds: timeBounds
            ? { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() }
            : undefined,
        },
      };
    }

    const manageDataOp = firstOpBody.manageDataOp()!;
    const dataName = manageDataOp.dataName.toString();
    const expectedDataName = `${homeDomain} auth`;

    if (dataName !== expectedDataName) {
      return {
        valid: false,
        error: `manage_data name mismatch: expected "${expectedDataName}", got "${dataName}"`,
        details: {
          sourceAccount: sourceAccountStr,
          sequence: sequenceStr,
          operations: operations.map((op) => ({
            type: op.body().switch().name,
            name: op.body().manageDataOp()?.dataName?.toString(),
            authRequired: op.body().manageDataOp() !== undefined,
          })),
          timeBounds: timeBounds
            ? { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() }
            : undefined,
        },
      };
    }

    // 5. Verify web_auth_domain - check if there's a second manage_data op with web_auth_domain
    // According to SEP-10, the challenge may include a second manage_data operation
    // with key "web_auth_domain" and value matching the auth endpoint host
    let webAuthDomainValid = false;
    for (let i = 1; i < operations.length; i++) {
      const op = operations[i];
      const opBody = op.body();
      if (opBody.switch().name === 'manageData') {
        const mdOp = opBody.manageDataOp()!;
        const name = mdOp.dataName.toString();
        if (name === 'web_auth_domain') {
          const value = mdOp.dataValue ? mdOp.dataValue.toString() : '';
          if (value === webAuthDomain) {
            webAuthDomainValid = true;
            break;
          } else {
            return {
              valid: false,
              error: `web_auth_domain mismatch: expected "${webAuthDomain}", got "${value}"`,
              details: {
                sourceAccount: sourceAccountStr,
                sequence: sequenceStr,
                operations: operations.map((op) => ({
                  type: op.body().switch().name,
                  name: op.body().manageDataOp()?.dataName?.toString(),
                  authRequired: op.body().manageDataOp() !== undefined,
                })),
                timeBounds: timeBounds
                  ? { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() }
                  : undefined,
              },
            };
          }
        }
      }
    }

    // If there are more than 1 operations, web_auth_domain should be present
    if (operations.length > 1 && !webAuthDomainValid) {
      return {
        valid: false,
        error: 'Missing web_auth_domain operation in challenge',
        details: {
          sourceAccount: sourceAccountStr,
          sequence: sequenceStr,
          operations: operations.map((op) => ({
            type: op.body().switch().name,
            name: op.body().manageDataOp()?.dataName?.toString(),
            authRequired: op.body().manageDataOp() !== undefined,
          })),
          timeBounds: timeBounds
            ? { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() }
            : undefined,
        },
      };
    }

    // 6. Verify time bounds are valid
    if (!timeBounds) {
      return {
        valid: false,
        error: 'Challenge transaction missing time bounds',
        details: {
          sourceAccount: sourceAccountStr,
          sequence: sequenceStr,
          operations: operations.map((op) => ({
            type: op.body().switch().name,
            name: op.body().manageDataOp()?.dataName?.toString(),
            authRequired: op.body().manageDataOp() !== undefined,
          })),
          timeBounds: undefined,
        },
      };
    }

    const minTime = Number(timeBounds.minTime());
    const maxTime = Number(timeBounds.maxTime());
    const now = Math.floor(Date.now() / 1000);

    if (minTime > maxTime) {
      return {
        valid: false,
        error: `Invalid time bounds: minTime (${minTime}) > maxTime (${maxTime})`,
        details: {
          sourceAccount: sourceAccountStr,
          sequence: sequenceStr,
          operations: operations.map((op) => ({
            type: op.body().switch().name,
            name: op.body().manageDataOp()?.dataName?.toString(),
            authRequired: op.body().manageDataOp() !== undefined,
          })),
          timeBounds: { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() },
        },
      };
    }

    if (now < minTime - maxTimeboundsAge) {
      return {
        valid: false,
        error: `Challenge not yet valid: current time ${now} < minTime ${minTime}`,
        details: {
          sourceAccount: sourceAccountStr,
          sequence: sequenceStr,
          operations: operations.map((op) => ({
            type: op.body().switch().name,
            name: op.body().manageDataOp()?.dataName?.toString(),
            authRequired: op.body().manageDataOp() !== undefined,
          })),
          timeBounds: { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() },
        },
      };
    }

    if (now > maxTime + maxTimeboundsAge) {
      return {
        valid: false,
        error: `Challenge expired: current time ${now} > maxTime ${maxTime}`,
        details: {
          sourceAccount: sourceAccountStr,
          sequence: sequenceStr,
          operations: operations.map((op) => ({
            type: op.body().switch().name,
            name: op.body().manageDataOp()?.dataName?.toString(),
            authRequired: op.body().manageDataOp() !== undefined,
          })),
          timeBounds: { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() },
        },
      };
    }

    // 7. Verify no unexpected operations (only manage_data allowed)
    for (let i = 0; i < operations.length; i++) {
      const op = operations[i];
      const opBody = op.body();
      if (opBody.switch().name !== 'manageData') {
        return {
          valid: false,
          error: `Unexpected operation type: ${opBody.switch().name} (only manage_data allowed)`,
          details: {
            sourceAccount: sourceAccountStr,
            sequence: sequenceStr,
            operations: operations.map((op) => ({
              type: op.body().switch().name,
              name: op.body().manageDataOp()?.dataName?.toString(),
              authRequired: op.body().manageDataOp() !== undefined,
            })),
            timeBounds: { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() },
          },
        };
      }
    }

    return {
      valid: true,
      details: {
        sourceAccount: sourceAccountStr,
        sequence: sequenceStr,
        operations: operations.map((op) => ({
          type: op.body().switch().name,
          name: op.body().manageDataOp()?.dataName?.toString(),
          authRequired: op.body().manageDataOp() !== undefined,
        })),
        timeBounds: { minTime: timeBounds.minTime().toString(), maxTime: timeBounds.maxTime().toString() },
      },
    };
  } catch (e) {
    return {
      valid: false,
      error: `Verification failed: ${(e as Error).message}`,
    };
  }
}

/**
 * Helper to create a verifyChallenge function pre-configured with server details.
 * Useful when the same server config is used across multiple calls.
 */
export function createChallengeVerifier(options: VerifyChallengeOptions) {
  return (challengeXdr: string) => verifyChallenge(challengeXdr, options);
}

export default verifyChallenge;