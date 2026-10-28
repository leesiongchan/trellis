import { expect, it, vi } from 'vite-plus/test';
import { createResearchSession } from '../../src/lib/knowledge/server-session.svelte';
import { createSessionClient as createLibraryClient } from '../fixtures/session-client';
import { library, snapshot, completed, jsonResponse } from '../fixtures/library';
it('a rejected revision removes the unadmitted intent and requires an explicit reload', async () => {
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
  const session = createResearchSession({ client, onChange: vi.fn() });
  await session.connect();
  await session.load('e1');
  await expect(session.refresh('root')).rejects.toMatchObject({ code: 'REVISION_CONFLICT' });
  expect(session.conflict).toBe(true);

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

it.each([
  ['Memory?', 'Memory'],
  ['Power at 1 mW', 'Power at 1 MW'],
  ['Storage at 1 MB', 'Storage at 1 Mb']
])(
  'different literal wording %s / %s never silently selects an existing sibling answer',
  async (existingQuestion, newQuestion) => {
    const saved = snapshot();
    saved.appearances[0].childIds = ['old-child'];
    saved.appearances.push({
      ...saved.appearances[0],
      id: 'old-child',
      parentId: 'root',
      topicId: 'old-topic',
      question: existingQuestion,
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
          expect(command.payload.question).toBe(newQuestion);
          saved.exploration.revision = 1;
          saved.appearances[0].childIds.push('new-child');
          saved.appearances.push({
            ...saved.appearances[1],
            id: 'new-child',
            topicId: 'new-topic',
            question: newQuestion,
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

      onChange: vi.fn()
    });
    await session.connect();
    await session.load('e1');
    const id = await session.addQuestion('root', newQuestion);
    expect(branchCalls).toBe(1);
    expect(id).toBe('new-child');
  }
);
