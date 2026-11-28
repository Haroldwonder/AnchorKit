/**
 * Off-chain attestation verification example (issue #1561).
 *
 * Uses the addresses and payloads from docs/test_vectors.md. Replace the two
 * `fetch*` stubs with `get_attestation` and `get_attestation_keys` calls over
 * Soroban RPC when verifying an attestation from a deployed contract.
 *
 * Run with: npx tsx examples/attestation_verification_example.ts
 */
import { createHash, createPrivateKey, createPublicKey, sign, verify } from "node:crypto";

const ADDR_A = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
const ADDR_B = "GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGZXG5CPCJDGX2VOTZ5JXN";
const DOMAIN = Buffer.from("anchorkit-attest-v1", "utf8");
const NETWORK_PASSPHRASE = "Standalone Network ; February 2017";
const CONTRACT_ID = "0000000000000000000000000000000000000000000000000000000000000001";

/** The six payload-hash vectors from docs/test_vectors.md. */
const vectors = [
  { name: "baseline", subject: ADDR_A, timestamp: 1700000000n, data: Buffer.from("kyc_approved", "utf8") },
  { name: "different timestamp", subject: ADDR_A, timestamp: 1n, data: Buffer.from("kyc_approved", "utf8") },
  { name: "empty data", subject: ADDR_A, timestamp: 1700000000n, data: Buffer.alloc(0) },
  { name: "max timestamp", subject: ADDR_A, timestamp: 18446744073709551615n, data: Buffer.from("payment_confirmed", "utf8") },
  { name: "different subject", subject: ADDR_B, timestamp: 1700000000n, data: Buffer.from("kyc_approved", "utf8") },
  { name: "binary data", subject: ADDR_A, timestamp: 0n, data: Buffer.from("00ff00ff00ff00ff", "hex") },
] as const;

type Attestation = {
  id: number;
  issuer: string;
  subject: string;
  timestamp: bigint;
  payloadHash: Buffer;
  signature: Buffer;
  issuerRevoked: boolean;
  expiresAt: bigint | null;
};

const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/** Decode a Stellar G-address into its 32-byte Ed25519 public key. */
function strkeyToRaw(strkey: string): Buffer {
  let accumulator = 0;
  let bits = 0;
  const bytes: number[] = [];
  for (const char of strkey) {
    const value = ALPHABET.indexOf(char);
    if (value < 0) continue; // checksum characters
    accumulator = (accumulator << 5) | value;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      bytes.push((accumulator >> bits) & 0xff);
    }
  }
  return Buffer.from(bytes.slice(1, 33)); // version byte + 32-byte key
}

/** ScAddress XDR: discriminant 0/1, 4 bytes of padding, then the 32-byte id. */
function accountXdr(strkey: string): Buffer {
  return Buffer.concat([Buffer.alloc(4), Buffer.alloc(4), strkeyToRaw(strkey)]);
}

function contractXdr(hexId: string): Buffer {
  return Buffer.concat([Buffer.from("00000001", "hex"), Buffer.from(hexId, "hex")]);
}

function u64be(value: bigint): Buffer {
  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64BE(value);
  return buffer;
}

function sha256(parts: Buffer[]): Buffer {
  return createHash("sha256").update(Buffer.concat(parts)).digest();
}

/** Mirrors anchorkit::compute_payload_hash. */
function payloadHash(subject: string, timestamp: bigint, data: Buffer): Buffer {
  return sha256([accountXdr(subject), u64be(timestamp), data]);
}

/** Mirrors the exported attestation_signing_digest helper. */
function signingDigest(issuer: string, subject: string, timestamp: bigint, hash: Buffer): Buffer {
  return sha256([
    DOMAIN,
    sha256([Buffer.from(NETWORK_PASSPHRASE, "utf8")]), // network id
    contractXdr(CONTRACT_ID),
    accountXdr(issuer),
    accountXdr(subject),
    u64be(timestamp),
    hash,
  ]);
}

// In production: client.get_attestation(id) and client.get_attestation_keys(issuer).
function fetchAttestation(vector: (typeof vectors)[number]): Attestation {
  const hash = payloadHash(vector.subject, vector.timestamp, vector.data);
  const privateKey = createPrivateKey({
    key: Buffer.concat([
      Buffer.from("302e020100300506032b657004220420", "hex"),
      Buffer.alloc(32, 7),
    ]),
    format: "der",
    type: "pkcs8",
  });
  return {
    id: 1,
    issuer: ADDR_A,
    subject: vector.subject,
    timestamp: vector.timestamp,
    payloadHash: hash,
    signature: sign(null, signingDigest(ADDR_A, vector.subject, vector.timestamp, hash), privateKey),
    issuerRevoked: false,
    expiresAt: 2100000000n,
  };
}

function fetchRegisteredKey(issuer: string): Buffer {
  const privateKey = createPrivateKey({
    key: Buffer.concat([
      Buffer.from("302e020100300506032b657004220420", "hex"),
      Buffer.alloc(32, 7),
    ]),
    format: "der",
    type: "pkcs8",
  });
  const spki = createPublicKey(privateKey).export({ format: "der", type: "spki" });
  if (issuer !== ADDR_A) throw new Error("unexpected issuer");
  return spki.subarray(spki.length - 32);
}

function verifyAttestation(attestation: Attestation, now: bigint): boolean {
  const digest = signingDigest(
    attestation.issuer,
    attestation.subject,
    attestation.timestamp,
    attestation.payloadHash,
  );
  const registeredKey = fetchRegisteredKey(attestation.issuer);
  const keyObject = createPublicKey({
    key: Buffer.concat([
      Buffer.from("302a300506032b6570032100", "hex"),
      registeredKey,
    ]),
    format: "der",
    type: "spki",
  });
  const signatureValid = verify(null, digest, keyObject, attestation.signature);
  const revoked = attestation.issuerRevoked;
  const expired = attestation.expiresAt !== null && now > attestation.expiresAt;
  console.log(
    `${signatureValid && !revoked && !expired ? "✓" : "✗"} ${attestation.id}: signature=${signatureValid} revoked=${revoked} expired=${expired}`,
  );
  return signatureValid && !revoked && !expired;
}

const results = vectors.map((vector) => {
  console.log(`${vector.name}: payload=${payloadHash(vector.subject, vector.timestamp, vector.data).toString("hex")}`);
  return verifyAttestation(fetchAttestation(vector), 1800000000n);
});

// Tampering with the subject invalidates the signature bound to the digest.
const original = fetchAttestation(vectors[0]);
const tampered: Attestation = { ...original, subject: ADDR_B };
const tamperedDigest = signingDigest(tampered.issuer, tampered.subject, tampered.timestamp, tampered.payloadHash);
const key = createPublicKey({
  key: Buffer.concat([
    Buffer.from("302a300506032b6570032100", "hex"),
    fetchRegisteredKey(tampered.issuer),
  ]),
  format: "der",
  type: "spki",
});
if (verify(null, tamperedDigest, key, tampered.signature)) {
  throw new Error("tampered subject unexpectedly verified");
}
console.log("✓ tampered subject rejected");

if (results.some((result) => !result)) process.exitCode = 1;
