import 'fake-indexeddb/auto';
import { expect, it, vi } from 'vitest';
import { createExplorationRepository } from '../../src/lib/knowledge/persistence';
import { createLibraryClient } from '../../src/lib/knowledge/library-client';
import { createLibraryCache } from '../../src/lib/knowledge/cache';
import { migrateLegacy } from '../../src/lib/knowledge/migration';
import { makeExploration } from '../fixtures/knowledge';
import { library, snapshot, jsonResponse } from '../fixtures/library';
it('migration_keeps_v1_original and legacy_edit_after_import_creates_copy', async () => {
  const repo = createExplorationRepository('migration-' + crypto.randomUUID());
  await repo.save(makeExploration(), 0);
  await repo.save(makeExploration({ id: 'demo', mode: 'demo' }), 0);
  const originalV1Before = await repo.list();
  const begins: unknown[] = [];
  let upload = '';
  const fetcher = vi.fn(async (path: RequestInfo | URL, init?: RequestInit) => {
    if (path === '/api/library') return jsonResponse(library);
    if (path === '/api/explorations/e1') return jsonResponse(snapshot());
    const command = JSON.parse(init!.body as string);
    if (path === '/api/imports') {
      begins.push(command.payload);
      upload = command.operationId;
    }
    return jsonResponse({
      id: 'import1',
      uploadOperationId: upload,
      explorationId: String(path).endsWith('/completion') ? 'e1' : null,
      state: String(path).endsWith('/completion') ? 'completed' : 'staging',
      mapping: { root: 'root' },
      serverRevision: 0,
      error: null
    });
  });
  const client = createLibraryClient(fetcher);
  await client.bootstrap();
  const cache = createLibraryCache('migration-cache-' + crypto.randomUUID());
  await cache.acceptLibrary(library);
  const first = await migrateLegacy(repo, client, cache);
  expect(first.imported).toBe(1);
  expect(first.mapping.e1).toEqual({ explorationId: 'e1', nodes: { root: 'root' } });
  const originalV1After = await repo.list();
  expect(originalV1After).toEqual(originalV1Before);
  const repeated = await migrateLegacy(repo, client, cache);
  expect(repeated.skipped).toBe(2);
  expect(begins).toHaveLength(1);
  const changed = (await repo.get('e1'))!;
  changed.nodes.root.content!.summary[0].text = 'Later original edit';
  await repo.save(changed, changed.revision);
  expect((await migrateLegacy(repo, client, cache)).imported).toBe(1);
  expect(begins).toHaveLength(2);
  expect((begins[0] as { contentDigest: string }).contentDigest).not.toBe(
    (begins[1] as { contentDigest: string }).contentDigest
  );
});

it('a lost snapshot read retries mapping and original view before acknowledging migration', async () => {
  const repo = createExplorationRepository('view-migration-' + crypto.randomUUID());
  const original = makeExploration();
  original.view.viewport.x = 77;
  await repo.save(original, 0);
  let failRead = true,
    upload = '';
  const client = createLibraryClient(
    vi.fn(async (path: RequestInfo | URL, init?: RequestInit) => {
      if (path === '/api/library') return jsonResponse(library);
      if (path === '/api/explorations/e1') {
        if (failRead) {
          failRead = false;
          throw new TypeError('Lost snapshot');
        }
        return jsonResponse(snapshot());
      }
      const command = JSON.parse(init!.body as string);
      if (path === '/api/imports') upload = command.operationId;
      return jsonResponse({
        id: 'import1',
        uploadOperationId: upload,
        explorationId: 'e1',
        state: 'completed',
        mapping: { root: 'root' },
        serverRevision: 0,
        error: null
      });
    })
  );
  await client.bootstrap();
  const cache = createLibraryCache('view-migration-cache-' + crypto.randomUUID());
  await cache.acceptLibrary(library);
  expect((await migrateLegacy(repo, client, cache)).failed).toBe(1);
  expect((await migrateLegacy(repo, client, cache)).imported).toBe(1);
  expect((await cache.getView(library, 'e1'))!.viewport.x).toBe(77);
});
