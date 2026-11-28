import { webcrypto } from 'crypto';
import { CallbackError, verifyAnchorCallback } from '../../src/webhook';

// eslint-disable-next-line @typescript-eslint/no-require-imports
const fixture = require('../../../tests/fixtures/anchor_callback.json') as {
  signing_key: string;
  timestamp: string;
  body: string;
  signature: string;
};

Object.defineProperty(global, 'crypto', { value: webcrypto, configurable: true });

const headers = {
  Signature: fixture.signature,
  Timestamp: fixture.timestamp,
};

describe('verifyAnchorCallback (#1573)', () => {
  it('accepts a correctly signed callback fixture', async () => {
    await expect(
      verifyAnchorCallback(headers, fixture.body, fixture.signing_key, 300, 1_700_000_060),
    ).resolves.toBeUndefined();
  });

  it('rejects a callback outside the allowed age', async () => {
    await expect(
      verifyAnchorCallback(headers, fixture.body, fixture.signing_key, 60, 1_700_000_100),
    ).rejects.toEqual(new CallbackError('Expired'));
  });

  it('rejects a tampered body', async () => {
    await expect(
      verifyAnchorCallback(headers, '{"id":"tx-999"}', fixture.signing_key, 300, 1_700_000_060),
    ).rejects.toEqual(new CallbackError('InvalidSignature'));
  });

  it('rejects missing headers', async () => {
    await expect(
      verifyAnchorCallback({ Timestamp: fixture.timestamp }, fixture.body, fixture.signing_key, 300, 1_700_000_060),
    ).rejects.toEqual(new CallbackError('MissingHeader'));
  });
});
