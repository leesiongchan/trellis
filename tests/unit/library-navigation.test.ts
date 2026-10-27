import 'fake-indexeddb/auto';
import { expect, it, vi } from 'vitest';
import { LibraryNavigation } from '../../src/lib/knowledge/library-navigation.svelte';
import { createLibraryClient } from '../../src/lib/knowledge/library-client';
import { createLibraryCache } from '../../src/lib/knowledge/cache';
import { createResearchSession } from '../../src/lib/knowledge/server-session.svelte';
import { library, snapshot, jsonResponse } from '../fixtures/library';
it('a late retained-topic read cannot replace a newer navigation', async () => {
  let release!: (response: Response) => void;
  const client = createLibraryClient(
    vi.fn(async (path) =>
      path === '/api/library'
        ? jsonResponse(library)
        : new Promise<Response>((resolve) => {
            release = resolve;
          })
    )
  );
  const session = createResearchSession({
    client,
    cache: createLibraryCache('navigation-' + crypto.randomUUID()),
    onChange: vi.fn()
  });
  await session.connect();
  const onNavigate = vi.fn(),
    navigation = new LibraryNavigation(client, session, onNavigate);
  const open = navigation.openTopic('topic1');
  await vi.waitFor(() => expect(release).toBeDefined());
  navigation.reset();
  const answer = snapshot().answers[0];
  release(
    jsonResponse({
      id: 'topic1',
      question: 'Saved topic',
      currentAnswerVersionId: answer.id,
      selectedAnswer: answer,
      versions: [{ id: answer.id, revision: answer.revision, createdAt: answer.createdAt }]
    })
  );
  await open;
  expect(navigation.retained).toBeNull();
  expect(onNavigate).not.toHaveBeenCalled();
  navigation.dispose();
});
it('a stale catalog response cannot replace a newer search', async () => {
  let release!: (response: Response) => void;
  const item = (id: string) => ({
    id,
    question: id,
    currentAnswerVersionId: null,
    answerVersionCount: 0,
    updatedAt: '2026-10-27T00:00:00Z'
  });
  const client = createLibraryClient(
    vi.fn(async (path) => {
      if (path === '/api/library') return jsonResponse(library);
      if (String(path).includes('search=old'))
        return new Promise<Response>((resolve) => {
          release = resolve;
        });
      return jsonResponse({ items: [item('new')], nextCursor: null });
    })
  );
  const session = createResearchSession({
    client,
    cache: createLibraryCache('catalog-' + crypto.randomUUID()),
    onChange: vi.fn()
  });
  await session.connect();
  const navigation = new LibraryNavigation(client, session, vi.fn());
  navigation.search = 'old';
  const old = navigation.listTopics();
  await vi.waitFor(() => expect(release).toBeDefined());
  navigation.search = 'new';
  await navigation.listTopics();
  release(jsonResponse({ items: [item('old')], nextCursor: null }));
  await old;
  expect(navigation.catalog.items[0].id).toBe('new');
  navigation.dispose();
});
