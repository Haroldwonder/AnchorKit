/**
 * SEP-6/24/31 anchor callback signature verification (#1573).
 *
 * The `Signature` header is a base64 ed25519 signature over the decimal
 * `Timestamp` header (unix seconds) followed by the raw request body, signed
 * with the anchor's stellar.toml `SIGNING_KEY` (a "G..." StrKey).
 */

export type CallbackErrorCode =
  | 'MissingHeader'
  | 'InvalidTimestamp'
  | 'Expired'
  | 'InvalidSigningKey'
  | 'InvalidSignature';

export class CallbackError extends Error {
  constructor(public readonly code: CallbackErrorCode) {
    super(code);
    this.name = 'CallbackError';
  }
}

const BASE32_ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';

function crc16Xmodem(bytes: Uint8Array): number {
  let crc = 0;
  for (const byte of bytes) {
    crc ^= byte << 8;
    for (let i = 0; i < 8; i += 1) {
      crc = crc & 0x8000 ? ((crc << 1) ^ 0x1021) & 0xffff : (crc << 1) & 0xffff;
    }
  }
  return crc;
}

/** Decode the 32-byte ed25519 key from a Stellar account StrKey. */
export function decodeSigningKey(signingKey: string): Uint8Array {
  if (signingKey.length !== 56) throw new CallbackError('InvalidSigningKey');
  let buffer = 0;
  let bits = 0;
  const decoded: number[] = [];
  for (const char of signingKey.toUpperCase()) {
    const value = BASE32_ALPHABET.indexOf(char);
    if (value < 0) throw new CallbackError('InvalidSigningKey');
    buffer = (buffer << 5) | value;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      decoded.push((buffer >> bits) & 0xff);
    }
  }
  if (decoded.length !== 35 || decoded[0] !== 6 << 3) {
    throw new CallbackError('InvalidSigningKey');
  }
  const expected = decoded[33] | (decoded[34] << 8);
  if (crc16Xmodem(Uint8Array.from(decoded.slice(0, 33))) !== expected) {
    throw new CallbackError('InvalidSigningKey');
  }
  return Uint8Array.from(decoded.slice(1, 33));
}

function base64ToBytes(value: string): Uint8Array {
  const binary = atob(value.trim());
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

function getHeader(
  headers: Record<string, string> | Array<[string, string]>,
  name: string,
): string | undefined {
  if (Array.isArray(headers)) {
    return headers.find(([key]) => key.toLowerCase() === name.toLowerCase())?.[1];
  }
  const entry = Object.keys(headers).find((key) => key.toLowerCase() === name.toLowerCase());
  return entry === undefined ? undefined : headers[entry];
}

/** Verify a signed anchor callback. Throws `CallbackError` on failure. */
export async function verifyAnchorCallback(
  headers: Record<string, string> | Array<[string, string]>,
  body: string | Uint8Array,
  signingKey: string,
  maxAge: number,
  nowSeconds: number = Math.floor(Date.now() / 1000),
): Promise<void> {
  const timestamp = getHeader(headers, 'Timestamp');
  const signature = getHeader(headers, 'Signature');
  if (!timestamp || !signature) throw new CallbackError('MissingHeader');
  if (!/^\d+$/.test(timestamp)) throw new CallbackError('InvalidTimestamp');
  const timestampSeconds = Number(timestamp);
  if (!Number.isSafeInteger(timestampSeconds) || nowSeconds - timestampSeconds > maxAge || nowSeconds < timestampSeconds) {
    throw new CallbackError('Expired');
  }

  const keyBytes = decodeSigningKey(signingKey);
  let key: CryptoKey;
  try {
    key = await crypto.subtle.importKey('raw', keyBytes, { name: 'Ed25519' }, false, ['verify']);
  } catch {
    throw new CallbackError('InvalidSigningKey');
  }

  const bodyBytes = typeof body === 'string' ? new TextEncoder().encode(body) : body;
  const signed = new Uint8Array(timestamp.length + bodyBytes.length);
  signed.set(new TextEncoder().encode(timestamp), 0);
  signed.set(bodyBytes, timestamp.length);

  let signatureBytes: Uint8Array;
  try {
    signatureBytes = base64ToBytes(signature);
  } catch {
    throw new CallbackError('InvalidSignature');
  }
  const valid = await crypto.subtle.verify('Ed25519', key, signatureBytes, signed);
  if (!valid) throw new CallbackError('InvalidSignature');
}
