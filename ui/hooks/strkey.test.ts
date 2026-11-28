import { isValidAttestor, isValidContract, isValidEd25519PublicKey } from './strkey';

// Checksum-valid StrKey fixtures (all-zero and all-0x01 payloads).
const G_ZERO = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
const G_ONES = 'GAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQDZ7H';
const C_ZERO = 'CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4';
const C_ONES = 'CAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQC526';

describe('isValidEd25519PublicKey', () => {
  test.each([G_ZERO, G_ONES])('accepts %s', (addr) => {
    expect(isValidEd25519PublicKey(addr)).toBe(true);
  });

  test('rejects a contract address', () => {
    expect(isValidEd25519PublicKey(C_ZERO)).toBe(false);
  });

  test('rejects a checksum-invalid address with a valid format', () => {
    expect(isValidEd25519PublicKey(G_ZERO.slice(0, -1) + 'G')).toBe(false);
    expect(isValidEd25519PublicKey('G' + 'A'.repeat(55))).toBe(false);
  });

  test.each([null, undefined, '', 'GABC', G_ZERO.toLowerCase(), G_ZERO + 'A'])(
    'rejects malformed input %p',
    (addr) => {
      expect(isValidEd25519PublicKey(addr as string | null | undefined)).toBe(false);
    },
  );
});

describe('isValidContract', () => {
  test.each([C_ZERO, C_ONES])('accepts %s', (addr) => {
    expect(isValidContract(addr)).toBe(true);
  });

  test('rejects an account address', () => {
    expect(isValidContract(G_ZERO)).toBe(false);
  });

  test('rejects a checksum-invalid address with a valid format', () => {
    expect(isValidContract(C_ONES.slice(0, -1) + '7')).toBe(false);
    expect(isValidContract('C' + 'A'.repeat(55))).toBe(false);
  });
});

describe('isValidAttestor', () => {
  test('accepts both account and contract addresses', () => {
    expect(isValidAttestor(G_ZERO)).toBe(true);
    expect(isValidAttestor(C_ZERO)).toBe(true);
  });

  test('rejects checksum-invalid addresses', () => {
    expect(isValidAttestor(G_ONES.slice(0, -1) + 'A')).toBe(false);
    expect(isValidAttestor(C_ZERO.slice(0, -1) + '5')).toBe(false);
  });
});
