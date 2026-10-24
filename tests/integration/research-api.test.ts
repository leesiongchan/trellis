import { expect, it, vi } from 'vitest';
import type { RequestEvent } from '@sveltejs/kit';
import { createResearchHandler } from '../../src/lib/server/research/handler';
import { createSessionToken } from '../../src/lib/server/access';
import { makeContent, makeRequest } from '../fixtures/knowledge';
const accessKey = 'a'.repeat(64),
  origin = 'http://127.0.0.1:5173';
const settings = () => ({
  openaiKey: 'test-openai',
  tavilyKey: 'test-search',
  model: 'test-model',
  origin,
  accessKey,
  limits: { bodyBytes: 65536, researchMs: 120000, searchMs: 20000, concurrent: 2, perMinute: 10 }
});
function event(
  body: unknown = makeRequest(),
  options: { origin?: string; cookie?: boolean; raw?: string } = {}
): RequestEvent {
  const request = new Request(origin + '/api/research', {
    method: 'POST',
    headers: { origin: options.origin ?? origin, 'content-type': 'application/json' },
    body: options.raw ?? JSON.stringify(body)
  });
  return {
    request,
    cookies: { get: () => (options.cookie === false ? undefined : createSessionToken(accessKey)) },
    locals: {}
  } as unknown as RequestEvent;
}
const dependencies = () => ({
  search: vi.fn(async () =>
    makeContent().sources.map((s) => ({ ...s, text: 'Flash retains data without power.' }))
  ),
  generate: vi.fn(async () => ({ kind: 'topic' as const, content: makeContent() })),
  verify: vi.fn(async () => ({ unsupportedParagraphIds: [], unsupportedBranchIndexes: [] }))
});
it('rejects anonymous and cross-origin requests before calling providers', async () => {
  const deps = dependencies(),
    handler = createResearchHandler(deps, settings);
  expect((await handler(event(undefined, { cookie: false }))).status).toBe(401);
  expect((await handler(event(undefined, { origin: 'https://hostile.example' }))).status).toBe(403);
  expect(deps.search).not.toHaveBeenCalled();
  const missing = event();
  missing.request.headers.delete('origin');
  expect((await handler(missing)).status).toBe(403);
  const host = event();
  host.request.headers.set('host', 'hostile.example');
  expect((await handler(host)).status).toBe(403);
});
it('times out a stuck provider and emits redacted correlated diagnostics', async () => {
  const logs = vi.spyOn(console, 'info').mockImplementation(() => {}),
    deps = dependencies();
  deps.search = vi.fn(() => new Promise(() => {}));
  const handler = createResearchHandler(deps, () => ({
    ...settings(),
    limits: { ...settings().limits, researchMs: 15 }
  }));
  const response = await handler(event());
  expect(response.status).toBe(504);
  const body = await response.json();
  const records = logs.mock.calls.map((c) => JSON.parse(c[0]));
  expect(
    records.some((r) => r.event === 'research_failed' && r.requestId === body.error.requestId)
  ).toBe(true);
  expect(JSON.stringify(records)).not.toContain('test-openai');
  expect(JSON.stringify(records)).not.toContain('microSD');
  logs.mockRestore();
});
it('validates input size, malformed JSON, blank questions and missing provider setup', async () => {
  const handler = createResearchHandler(dependencies(), settings);
  expect((await handler(event(undefined, { raw: 'x'.repeat(65537) }))).status).toBe(413);
  expect((await handler(event(undefined, { raw: '{bad' }))).status).toBe(400);
  expect((await handler(event(makeRequest({ question: ' ' })))).status).toBe(422);
  const notConfigured = createResearchHandler(dependencies(), () => ({
    ...settings(),
    openaiKey: ''
  }));
  const response = await notConfigured(event());
  expect(response.status).toBe(503);
  expect(JSON.stringify(await response.json())).not.toContain('test-search');
});
it('returns one consistent error envelope without upstream internals', async () => {
  const deps = dependencies();
  deps.search.mockRejectedValueOnce(new Error('secret-provider-token'));
  const response = await createResearchHandler(deps, settings)(event());
  expect(response.status).toBe(502);
  const body = await response.json();
  expect(body.error.requestId).toBeTruthy();
  expect(JSON.stringify(body)).not.toContain('secret-provider-token');
});
it('shares identical concurrent work and enforces limits for distinct jobs', async () => {
  const deps = dependencies();
  let finish!: (value: ReturnType<typeof makeContent>) => void;
  deps.generate = vi.fn(async () => ({
    kind: 'topic' as const,
    content: await new Promise<ReturnType<typeof makeContent>>((r) => (finish = r))
  }));
  const handler = createResearchHandler(deps, () => ({
    ...settings(),
    limits: { ...settings().limits, concurrent: 1 }
  }));
  const a = handler(event()),
    b = handler(event());
  await vi.waitFor(() => expect(deps.generate).toHaveBeenCalledTimes(1));
  expect((await handler(event(makeRequest({ nodeId: 'different' })))).status).toBe(429);
  finish(makeContent());
  expect((await a).status).toBe(200);
  expect((await b).status).toBe(200);
});
