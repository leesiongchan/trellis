import 'fake-indexeddb/auto';
import { expect, it, vi } from 'vitest';
import { createResearchSession } from '../../src/lib/knowledge/server-session.svelte';
import { createLibraryClient } from '../../src/lib/knowledge/library-client';
import { createLibraryCache } from '../../src/lib/knowledge/cache';
import { library, snapshot, completed, jsonResponse } from '../fixtures/library';
it('retry_preserves_intent_id and resolves it before creating new work', async () => {
  const submissions: unknown[] = [];
  let lost = true;
  const fetcher = vi.fn(async (path: RequestInfo | URL, init?: RequestInit) => {
    if (path === '/api/library') return jsonResponse(library);
    if (String(path).startsWith('/api/operations/'))
      return jsonResponse(
        { error: { code: 'NOT_FOUND', message: 'Not found', retryable: false, requestId: 'test' } },
        404
      );
    if (path === '/api/explorations' && init?.method === 'POST') {
      const command = JSON.parse(init.body as string);
      submissions.push(command);
      if (lost) {
        lost = false;
        throw new TypeError('Response lost');
      }
      return jsonResponse(completed(command.operationId));
    }
    return jsonResponse(snapshot());
  });
  const cacheName = 'session-' + crypto.randomUUID();
  const cache = createLibraryCache(cacheName);
  const session = createResearchSession({
    client: createLibraryClient(fetcher),
    cache,
    onChange: vi.fn()
  });
  await session.connect();
  await expect(session.start('microSD cards')).rejects.toThrow();
  expect(await cache.intents(library)).toHaveLength(1);
  session.dispose();
  const restored = createResearchSession({
    client: createLibraryClient(fetcher),
    cache: createLibraryCache(cacheName),
    onChange: vi.fn()
  });
  await restored.connect();
  const [first, retry] = submissions as { operationId: string }[];
  expect(retry.operationId).toBe(first.operationId);
  expect(retry).toEqual(first);
  expect(await cache.intents(library)).toHaveLength(0);
  expect(restored.exploration!.nodes.root.content!.title).toBe('microSD cards');
  expect(restored.exploration).not.toHaveProperty('schemaVersion');
});
it('offline_cache_is_read_only', async () => {
  const cache = createLibraryCache('offline-' + crypto.randomUUID());
  await cache.acceptLibrary(library);
  await cache.putSnapshot(library, snapshot());
  const offlineMutationRequests: unknown[] = [];
  const fetcher = vi.fn(async (_path: RequestInfo | URL, init?: RequestInit) => {
    if (init?.method && init.method !== 'GET') offlineMutationRequests.push(init);
    throw new TypeError('Offline');
  });
  const session = createResearchSession({
    client: createLibraryClient(fetcher),
    cache,
    onChange: vi.fn()
  });
  await session.connect();
  await session.load('e1');
  expect(session.offline).toBe(true);
  expect(session.exploration!.nodes.root.content).not.toBeNull();
  await expect(session.refresh('root')).rejects.toThrow(/offline/i);
  expect(offlineMutationRequests).toHaveLength(0);
  expect(await cache.intents(library)).toHaveLength(0);
});

it('a rejected revision removes the unadmitted intent and requires an explicit reload', async () => {
  const cache = createLibraryCache('conflict-' + crypto.randomUUID());
  const client = createLibraryClient(
    vi.fn(async (path: RequestInfo | URL, init?: RequestInit) => {
      if (path === '/api/library') return jsonResponse(library);
      if (init?.method === 'POST')
        return jsonResponse(
          {
            error: {
              code: 'REVISION_CONFLICT',
              message: 'Reload the saved version.',
              retryable: false,
              requestId: 'test'
            }
          },
          409
        );
      return jsonResponse(snapshot());
    })
  );
  const session = createResearchSession({ client, cache, onChange: vi.fn() });
  await session.connect();
  await session.load('e1');
  await expect(session.refresh('root')).rejects.toMatchObject({ code: 'REVISION_CONFLICT' });
  expect(session.conflict).toBe(true);
  expect(await cache.intents(library)).toHaveLength(0);
  await session.load('e1');
  expect(session.conflict).toBe(false);
});

it('stale exploration reads cannot replace a newer selection', async () => {
  let release!: (value: Response) => void;
  const client = createLibraryClient(
    vi.fn(async (path: RequestInfo | URL) => {
      if (path === '/api/library') return jsonResponse(library);
      if (path === '/api/explorations/e1')
        return new Promise<Response>((resolve) => {
          release = resolve;
        });
      const other = snapshot();
      other.exploration.id = 'e2';
      other.appearances[0].explorationId = 'e2';
      return jsonResponse(other);
    })
  );
  const session = createResearchSession({
    client,
    cache: createLibraryCache('stale-' + crypto.randomUUID()),
    onChange: vi.fn()
  });
  await session.connect();
  const old = session.load('e1');
  await vi.waitFor(() => expect(release).toBeDefined());
  await session.load('e2');
  release(jsonResponse(snapshot()));
  await old;
  expect(session.exploration!.id).toBe('e2');
});

it('disposal stops pending polling while retaining the admitted intent', async () => {
  let reads = 0;
  const pending = { ...completed('pending'), state: 'pending', result: null };
  const fetcher = vi.fn(async (path: RequestInfo | URL, init?: RequestInit) => {
    if (path === '/api/library') return jsonResponse(library);
    if (String(path).startsWith('/api/operations/')) {
      reads++;
      return jsonResponse(pending);
    }
    if (init?.method === 'POST')
      return jsonResponse({ ...pending, id: JSON.parse(init.body as string).operationId });
    return jsonResponse(snapshot());
  });
  const cache = createLibraryCache('dispose-' + crypto.randomUUID());
  const session = createResearchSession({
    client: createLibraryClient(fetcher),
    cache,
    onChange: vi.fn()
  });
  await session.connect();
  await session.load('e1');
  const request = session.refresh('root').catch(() => {});
  await vi.waitFor(() =>
    expect(fetcher.mock.calls.some((call) => call[1]?.method === 'POST')).toBe(true)
  );
  session.dispose();
  await request;
  const before = reads;
  await new Promise((resolve) => setTimeout(resolve, 300));
  expect(reads).toBe(before);
  expect(await cache.intents(library)).toHaveLength(1);
});

it('serializes local view writes and flushes the latest viewport before switching', async () => {
  const cache = createLibraryCache('view-order-' + crypto.randomUUID());
  const put = cache.putView;
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  let count = 0;
  cache.putView = vi.fn(async (...args: Parameters<typeof put>) => {
    if (++count === 1) await gate;
    return put(...args);
  });
  const client = createLibraryClient(
    vi.fn(async (path) => jsonResponse(path === '/api/library' ? library : snapshot()))
  );
  const session = createResearchSession({ client, cache, onChange: vi.fn() });
  await session.connect();
  await session.load('e1');
  session.setView({ viewport: { x: 1, y: 0, zoom: 1 } });
  session.setView({ viewport: { x: 2, y: 0, zoom: 1 } });
  await vi.waitFor(() => expect(count).toBeGreaterThan(0));
  expect(count).toBe(1);
  release();
  await session.flushView();
  expect((await cache.getView(library, 'e1'))!.viewport.x).toBe(2);
});

it('opening a newly pinned root materializes saved suggestions with refresh false', async () => {
  const requests: {
    path: string;
    command: { operationId: string; payload: { refresh?: boolean } };
  }[] = [];
  const saved = snapshot();
  const client = createLibraryClient(
    vi.fn(async (path: RequestInfo | URL, init?: RequestInit) => {
      if (path === '/api/library') return jsonResponse(library);
      if (init?.method === 'POST') {
        const command = JSON.parse(init.body as string);
        requests.push({ path: String(path), command });
        return jsonResponse(completed(command.operationId));
      }
      return jsonResponse(saved);
    })
  );
  const session = createResearchSession({
    client,
    cache: createLibraryCache('reuse-root-' + crypto.randomUUID()),
    onChange: vi.fn()
  });
  await session.connect();
  await session.start('microSD cards');
  const reuse = requests.filter((request) => request.path.endsWith('/research'));
  expect(reuse).toHaveLength(1);
  expect(reuse[0].command.payload.refresh).toBe(false);
});

it('completed intents for subsequently deleted explorations are retired without resubmission', async () => {
  const cache = createLibraryCache('deleted-intent-' + crypto.randomUUID());
  await cache.acceptLibrary(library);
  await cache.putIntent({
    library,
    path: '/api/explorations',
    method: 'POST',
    command: {
      operationId: 'done',
      libraryEpoch: 1,
      expectedRevision: null,
      payload: { question: 'q', locale: 'en' }
    }
  });
  const writes: unknown[] = [];
  const client = createLibraryClient(
    vi.fn(async (path: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method && init.method !== 'GET') writes.push(init);
      if (path === '/api/library') return jsonResponse(library);
      if (path === '/api/operations/done') return jsonResponse(completed('done'));
      return jsonResponse(
        {
          error: {
            code: 'NOT_FOUND',
            message: 'The exploration was deleted.',
            retryable: false,
            requestId: 'test'
          }
        },
        404
      );
    })
  );
  const session = createResearchSession({ client, cache, onChange: vi.fn() });
  await session.connect();
  expect(await cache.intents(library)).toHaveLength(0);
  expect(session.exploration).toBeNull();
  expect(session.notice).toMatch(/deleted/i);
  expect(writes).toHaveLength(0);
});

it('a late mutation conflict cannot mark a newly loaded exploration as stale', async () => {
  let release!: (response: Response) => void;
  const client = createLibraryClient(
    vi.fn(async (path: RequestInfo | URL, init?: RequestInit) => {
      if (path === '/api/library') return jsonResponse(library);
      if (init?.method === 'POST')
        return new Promise<Response>((resolve) => {
          release = resolve;
        });
      const value = snapshot();
      if (path === '/api/explorations/e2') {
        value.exploration.id = 'e2';
        value.appearances[0].explorationId = 'e2';
      }
      return jsonResponse(value);
    })
  );
  const session = createResearchSession({
    client,
    cache: createLibraryCache('late-conflict-' + crypto.randomUUID()),
    onChange: vi.fn()
  });
  await session.connect();
  await session.load('e1');
  const refresh = session.refresh('root').catch(() => {});
  await vi.waitFor(() => expect(release).toBeDefined());
  await session.load('e2');
  release(
    jsonResponse(
      {
        error: {
          code: 'REVISION_CONFLICT',
          message: 'Reload the earlier exploration.',
          retryable: false,
          requestId: 'test'
        }
      },
      409
    )
  );
  await refresh;
  expect(session.exploration!.id).toBe('e2');
  expect(session.conflict).toBe(false);
});

it('disposal while persisting an intent cannot submit work using a replacement signal', async () => {
  const cache = createLibraryCache('persist-cancel-' + crypto.randomUUID());
  const put = cache.putIntent;
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  let storing = false;
  cache.putIntent = async (intent) => {
    await put(intent);
    storing = true;
    await gate;
  };
  let writes = 0;
  const client = createLibraryClient(
    vi.fn(async (path: RequestInfo | URL, init?: RequestInit) => {
      if (path === '/api/library') return jsonResponse(library);
      if (init?.method === 'POST') {
        writes++;
        return jsonResponse(completed(JSON.parse(init.body as string).operationId));
      }
      return jsonResponse(snapshot());
    })
  );
  const session = createResearchSession({ client, cache, onChange: vi.fn() });
  await session.connect();
  await session.load('e1');
  const refresh = session.refresh('root').catch(() => {});
  await vi.waitFor(() => expect(storing).toBe(true));
  session.dispose();
  release();
  await refresh;
  expect(writes).toBe(0);
  expect(await cache.intents(library)).toHaveLength(1);
});
it('different literal punctuation never silently selects an existing sibling answer', async () => {
  const saved = snapshot();
  saved.appearances[0].childIds = ['old-child'];
  saved.appearances.push({
    ...saved.appearances[0],
    id: 'old-child',
    parentId: 'root',
    topicId: 'old-topic',
    question: 'Memory?',
    answerVersionId: null,
    childIds: []
  });
  let branchCalls = 0;
  const client = createLibraryClient(
    vi.fn(async (path: RequestInfo | URL, init?: RequestInit) => {
      if (path === '/api/library') return jsonResponse(library);
      if (init?.method === 'POST') {
        branchCalls++;
        const command = JSON.parse(init.body as string);
        expect(command.payload.question).toBe('Memory');
        saved.exploration.revision = 1;
        saved.appearances[0].childIds.push('new-child');
        saved.appearances.push({
          ...saved.appearances[1],
          id: 'new-child',
          topicId: 'new-topic',
          question: 'Memory',
          answerVersionId: 'new-answer'
        });
        saved.answers.push({ ...saved.answers[0], id: 'new-answer', topicId: 'new-topic' });
        return jsonResponse({
          ...completed(command.operationId),
          result: {
            kind: 'exploration',
            explorationId: 'e1',
            appearanceId: 'new-child',
            revision: 1
          }
        });
      }
      return jsonResponse(saved);
    })
  );
  const session = createResearchSession({
    client,
    cache: createLibraryCache('literal-sibling-' + crypto.randomUUID()),
    onChange: vi.fn()
  });
  await session.connect();
  await session.load('e1');
  const id = await session.addQuestion('root', 'Memory');
  expect(branchCalls).toBe(1);
  expect(id).toBe('new-child');
});
