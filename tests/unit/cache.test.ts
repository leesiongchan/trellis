import 'fake-indexeddb/auto';
import { expect, it } from 'vitest';
import { createLibraryCache } from '../../src/lib/knowledge/cache';
import { createExplorationRepository } from '../../src/lib/knowledge/persistence';
import { makeExploration } from '../fixtures/knowledge';
import { library, snapshot } from '../fixtures/library';
it('purged_or_other_library_cache_is_not_reused', async () => {
  const original = createExplorationRepository('original-' + crypto.randomUUID());
  await original.save(makeExploration(), 0);
  const cache = createLibraryCache('cache-' + crypto.randomUUID());
  await cache.acceptLibrary(library);
  await cache.putSnapshot(library, snapshot());
  expect(await cache.getSnapshot(library, 'e1')).not.toBeNull();
  await cache.acceptLibrary({ ...library, libraryEpoch: 2 });
  expect(await cache.getSnapshot(library, 'e1')).toBeNull();
  await cache.acceptLibrary({ ...library, libraryId: 'other' });
  expect(await cache.getSnapshot({ ...library, libraryId: 'other' }, 'e1')).toBeNull();
  expect((await original.list())[0].id).toBe('e1');
});
it('persists intents and local view independently from acknowledged content', async () => {
  const name = 'cache-' + crypto.randomUUID(),
    cache = createLibraryCache(name);
  await cache.acceptLibrary(library);
  await cache.putSnapshot(library, snapshot());
  const intent = {
    library,
    path: '/api/explorations',
    method: 'POST',
    command: {
      operationId: 'intent',
      libraryEpoch: 1,
      expectedRevision: null,
      payload: { question: 'q', locale: 'en' }
    }
  };
  await cache.putIntent(intent);
  expect(await createLibraryCache(name).intents(library)).toEqual([intent]);
  await cache.putView(library, 'e1', {
    selectedNodeId: 'root',
    expandedNodeIds: [],
    viewport: { x: 15, y: 20, zoom: 0.8 },
    mobilePane: 'reader'
  });
  expect((await cache.getSnapshot(library, 'e1'))!.exploration.revision).toBe(0);
  expect((await cache.getView(library, 'e1'))!.viewport.x).toBe(15);
});
