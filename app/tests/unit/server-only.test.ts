import { expect, it, vi } from 'vite-plus/test';
import { createResearchSession } from '../../src/lib/knowledge/server-session.svelte';
import { createLibraryClient } from '../../src/lib/knowledge/library-client';
import { library, snapshot, completed, jsonResponse } from '../fixtures/library';

const view = {
  selectedNodeId: 'root',
  expandedNodeIds: ['root'],
  viewport: { x: 42, y: -12, zoom: 0.75 },
  mobilePane: 'reader' as const
};
function setup(
  overrides: (path: string, init?: RequestInit) => Response | Promise<Response> | undefined = () =>
    undefined
) {
  const fetcher = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const path = String(input);
    const result = overrides(path, init);
    if (result !== undefined) return result;
    if (path === '/api/library') return jsonResponse(library);
    if (path.startsWith('/api/operations?')) return jsonResponse({ items: [], nextCursor: null });
    if (path.endsWith('/view'))
      return jsonResponse(
        init?.method === 'PUT' ? JSON.parse(init.body as string).payload.view : view
      );
    if (init?.method === 'POST')
      return jsonResponse(completed(JSON.parse(init.body as string).operationId));
    return jsonResponse(snapshot());
  });
  const session = createResearchSession({
    client: createLibraryClient(fetcher),
    onChange: vi.fn()
  });
  return { session, fetcher };
}

it('restores and saves exploration view through the server without browser storage', async () => {
  const { session, fetcher } = setup();
  await session.connect();
  await session.load('e1');
  expect(session.exploration!.view).toEqual(view);
  session.setView({ viewport: { x: 100, y: 20, zoom: 0.5 } });
  await session.flushView();
  const write = fetcher.mock.calls.find(
    ([path, init]) => String(path).endsWith('/view') && init?.method === 'PUT'
  )!;
  expect(JSON.parse(write[1]!.body as string).payload.view.viewport.x).toBe(100);
  expect(session.viewError).toBe('');
});

it('coalesces intermediate viewport positions into one server save when flushed', async () => {
  const { session, fetcher } = setup();
  await session.connect();
  await session.load('e1');
  for (const x of [10, 20, 30]) {
    session.setView({ viewport: { x, y: 0, zoom: 1 } });
    await Promise.resolve();
  }
  await session.flushView();
  const writes = fetcher.mock.calls.filter(([, init]) => init?.method === 'PUT');
  expect(writes).toHaveLength(1);
  expect(JSON.parse(writes[0][1]!.body as string).payload.view.viewport.x).toBe(30);
});

it('failed view saves block switching until a successful retry', async () => {
  let fail = true;
  const { session } = setup((path, init) => {
    if (path.endsWith('/view') && init?.method === 'PUT' && fail)
      return Promise.reject(new TypeError('Disconnected'));
  });
  await session.connect();
  await session.load('e1');
  session.setView({ mobilePane: 'map' });
  await expect(session.load('e2')).rejects.toThrow(/reach/i);
  expect(session.exploration!.id).toBe('e1');
  expect(session.viewError).not.toBe('');
  expect(session.readOnly).toBe(true);
  fail = false;
  await session.retryView();
  expect(session.viewError).toBe('');
});

it('discarding an exploration clears its rejected view save so navigation can continue', async () => {
  const { session } = setup((path, init) => {
    if (path.endsWith('/view') && init?.method === 'PUT')
      return Promise.reject(new TypeError('Disconnected'));
  });
  await session.connect();
  await session.load('e1');
  session.setView({ mobilePane: 'map' });
  await expect(session.flushView()).rejects.toThrow();
  session.dispose();
  session.exploration = null;
  await expect(session.flushView()).resolves.toBeUndefined();
  expect(session.viewError).toBe('');
});

it('an exploration read disconnection makes the existing view read-only', async () => {
  let fail = false;
  const { session } = setup((path) => {
    if (fail && path === '/api/explorations/e2')
      return Promise.reject(new TypeError('Disconnected'));
  });
  await session.connect();
  await session.load('e1');
  fail = true;
  await expect(session.load('e2')).rejects.toThrow();
  expect(session.readOnly).toBe(true);
  expect(session.exploration!.id).toBe('e1');
});

it('a fresh session cannot restore content when the server is unreachable', async () => {
  const { session } = setup(() => Promise.reject(new TypeError('Offline')));
  await expect(session.connect()).rejects.toMatchObject({ code: 'OFFLINE' });
  expect(session.exploration).toBeNull();
  expect(session.library).toBeNull();
  expect(session.readOnly).toBe(true);
});

it('recovers admitted pending operations from the server after a fresh session', async () => {
  const pending = { ...completed('saved-operation'), state: 'pending', result: null };
  let finished = false;
  const { session, fetcher } = setup((path) => {
    if (path.startsWith('/api/operations?'))
      return jsonResponse({ items: finished ? [] : [pending], nextCursor: null });
    if (path === '/api/operations/saved-operation') {
      finished = true;
      return jsonResponse(completed('saved-operation'));
    }
  });
  await session.connect();
  expect(session.exploration!.id).toBe('e1');
  expect(session.exploration!.nodes.root.content).not.toBeNull();
  expect(fetcher.mock.calls.some(([, init]) => init?.method === 'POST')).toBe(false);
});

it('retries an uncertain same-tab request with its original operation ID', async () => {
  let lost = true;
  const commands: unknown[] = [];
  const { session } = setup((path, init) => {
    if (path.startsWith('/api/operations/'))
      return jsonResponse(
        {
          error: { code: 'NOT_FOUND', message: 'Not admitted', retryable: false, requestId: 'test' }
        },
        404
      );
    if (path === '/api/explorations' && init?.method === 'POST') {
      commands.push(JSON.parse(init.body as string));
      if (lost) {
        lost = false;
        return Promise.reject(new TypeError('Response lost'));
      }
      return jsonResponse(completed(JSON.parse(init.body as string).operationId));
    }
  });
  await session.connect();
  await expect(session.start('microSD cards')).rejects.toThrow();
  await session.connect();
  expect(commands).toHaveLength(2);
  expect(commands[0]).toEqual(commands[1]);
  expect(session.exploration!.id).toBe('e1');
});
