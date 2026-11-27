/**
 * Stellar StrKey validation shared by every hook and component that accepts
 * an attestor / anchor address.
 *
 * Mirrors `StrKey.isValidEd25519PublicKey` and `StrKey.isValidContract` from
 * `@stellar/stellar-sdk`: the address is Base32-decoded, the version byte is
 * checked and the trailing CRC16-XModem checksum is verified.  A pure format
 * check (`/^[GC][A-Z2-7]{55}$/`) lets typos through that keep the shape but
 * break the checksum, which then only fail later at the RPC.
 *
 * Implemented without the SDK so the UI package keeps zero runtime
 * dependencies on it; the algorithm is identical.
 */

const BASE32_ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';

/** StrKey version bytes (see stellar-base `versionBytes`). */
const VERSION_ED25519_PUBLIC_KEY = 6 << 3; // 'G'
const VERSION_CONTRACT = 2 << 3; // 'C'

/** 1 version byte + 32 payload bytes + 2 checksum bytes = 35 bytes = 56 chars. */
const ADDRESS_LENGTH = 56;
const PAYLOAD_LENGTH = 32;

function base32Decode(input: string): Uint8Array | null {
  const out: number[] = [];
  let bits = 0;
  let value = 0;
  for (const ch of input) {
    const idx = BASE32_ALPHABET.indexOf(ch);
    if (idx === -1) return null;
    value = (value << 5) | idx;
    bits += 5;
    if (bits >= 8) {
      out.push((value >>> (bits - 8)) & 0xff);
      bits -= 8;
    }
  }
  // Any leftover bits must be zero padding for a canonical encoding.
  if (bits > 0 && (value & ((1 << bits) - 1)) !== 0) return null;
  return Uint8Array.from(out);
}

function crc16XModem(data: Uint8Array): number {
  let crc = 0x0000;
  for (const byte of data) {
    crc ^= byte << 8;
    for (let i = 0; i < 8; i++) {
      crc = crc & 0x8000 ? ((crc << 1) ^ 0x1021) & 0xffff : (crc << 1) & 0xffff;
    }
  }
  return crc;
}

function isValidStrKey(address: unknown, versionByte: number): boolean {
  if (typeof address !== 'string' || address.length !== ADDRESS_LENGTH) return false;

  const decoded = base32Decode(address);
  if (!decoded || decoded.length !== 1 + PAYLOAD_LENGTH + 2) return false;
  if (decoded[0] !== versionByte) return false;

  const data = decoded.subarray(0, decoded.length - 2);
  const expected = crc16XModem(data);
  // Checksum is stored little-endian.
  const actual = decoded[decoded.length - 2] | (decoded[decoded.length - 1] << 8);
  return expected === actual;
}

/** Equivalent of `StrKey.isValidEd25519PublicKey` (G-address). */
export function isValidEd25519PublicKey(address: string | null | undefined): boolean {
  return isValidStrKey(address, VERSION_ED25519_PUBLIC_KEY);
}

/** Equivalent of `StrKey.isValidContract` (C-address). */
export function isValidContract(address: string | null | undefined): boolean {
  return isValidStrKey(address, VERSION_CONTRACT);
}

/**
 * Accepts a checksum-valid Stellar account (G…) or contract (C…) address.
 * This is the single attestor validator used across hooks and components.
 */
export function isValidAttestor(addr: string | null | undefined): addr is string {
  return isValidEd25519PublicKey(addr) || isValidContract(addr);
}
