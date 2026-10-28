import { expect, it, vi } from 'vite-plus/test';
import { createLibraryClient } from '../../src/lib/knowledge/library-client';
import { library, completed, jsonResponse } from '../fixtures/library';
it('sends protocol and library identity, validates data and preserves supplied intent', async () => {
  const fetcher = vi.fn(async (path: RequestInfo | URL, _init?: RequestInit) =>
    jsonResponse(String(path) === '/api/library' ? library : completed('intent'))
  );
  const client = createLibraryClient(fetcher);
  await client.bootstrap();
  const command = {
    operationId: 'intent',
    libraryEpoch: 1,
    expectedRevision: null,
    payload: { question: 'Stored question', locale: 'en' }
  };
  await client.create(command);
  const init = fetcher.mock.calls[1][1]!;
  expect(new Headers(init.headers).get('X-Branch-Library')).toBe('library-a');
  expect(new Headers(init.headers).get('X-Branch-Protocol')).toBe('2');
  expect(JSON.parse(init.body as string)).toEqual(command);
});
it('rejects mismatched response protocol and malformed snapshots', async () => {
  const badProtocol = createLibraryClient(vi.fn(async () => new Response(JSON.stringify(library))));
  await expect(badProtocol.bootstrap()).rejects.toMatchObject({ code: 'RELOAD_REQUIRED' });
  const client = createLibraryClient(
    vi.fn(async (path) => jsonResponse(path === '/api/library' ? library : { schemaVersion: 1 }))
  );
  await client.bootstrap();
  await expect(client.exploration('e1')).rejects.toThrow();
});
