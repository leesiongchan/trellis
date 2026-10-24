import { expect, it } from 'vitest';
import { authorizeRequest, createSessionToken, validAccessKey } from '../../src/lib/server/access';
const key = 'secret'.repeat(12),
  request = new Request('https://example.org/api/research');
it('requires a signed, unexpired capability and validates the access link key', () => {
  expect(validAccessKey(key, key)).toBe(true);
  expect(validAccessKey('wrong', key)).toBe(false);
  expect(validAccessKey('', '')).toBe(false);
  expect(authorizeRequest(request, createSessionToken(key), key)).toBe(true);
  expect(authorizeRequest(request, createSessionToken('other'), key)).toBe(false);
  expect(authorizeRequest(request, createSessionToken(key, Date.now() - 1000), key)).toBe(false);
});
