import 'fake-indexeddb/auto';
import { expect, it, vi } from 'vitest';
import { openDB } from 'idb';
import {
  createAutosave,
  createExplorationRepository,
  restoreExploration
} from '../../src/lib/knowledge/persistence';
import { makeExploration } from '../fixtures/knowledge';

it('lists healthy explorations and reports unreadable records without altering them', async () => {
  const name = 'corrupt-' + crypto.randomUUID();
  const report = vi.fn();
  const repo = createExplorationRepository(name, report);
  const healthy = await repo.save(makeExploration(), 0);
  const raw = await openDB(name, 1);
  const unreadable = { id: 'future-data', schemaVersion: 999, privateData: 'preserve me' };
  await raw.put('explorations', unreadable);
  expect(await repo.list()).toEqual([healthy]);
  expect(report).toHaveBeenCalledWith(expect.stringMatching(/could not be read/i));
  expect(await raw.get('explorations', unreadable.id)).toEqual(unreadable);
  raw.close();
});

it('uses atomic revision checks and explicit deletion', async () => {
  const repo = createExplorationRepository('test-' + crypto.randomUUID());
  const value = makeExploration();
  const first = await repo.save(value, 0);
  expect(first.revision).toBe(1);
  const attempts = await Promise.allSettled([
    repo.save({ ...first, originalPrompt: 'A' }, 1),
    repo.save({ ...first, originalPrompt: 'B' }, 1)
  ]);
  expect(attempts.filter((a) => a.status === 'fulfilled')).toHaveLength(1);
  expect((await repo.get(value.id))!.revision).toBe(2);
  await repo.remove(value.id);
  expect(await repo.get(value.id)).toBe(null);
});
it('restores interrupted research without restarting it and preserves ready refresh content', () => {
  const value = makeExploration();
  value.nodes.root.status = 'researching';
  value.nodes.root.content = null;
  const restored = restoreExploration(value);
  expect(restored.nodes.root.status).toBe('failed');
  expect(restored.nodes.root.lastError?.retryable).toBe(true);
  const refreshing = makeExploration();
  refreshing.nodes.root.isRefreshing = true;
  expect(restoreExploration(refreshing).nodes.root.content).toEqual(refreshing.nodes.root.content);
  expect(restoreExploration(refreshing).nodes.root.isRefreshing).toBe(false);
  expect(() => restoreExploration({ ...value, schemaVersion: 999 })).toThrow();
});
it('serializes autosaves, reports quota failures, and never labels failures saved', async () => {
  const repo = createExplorationRepository('autosave-' + crypto.randomUUID()),
    status = vi.fn();
  const saver = createAutosave(repo, status);
  const value = makeExploration();
  saver.enqueue(value);
  saver.enqueue({ ...value, originalPrompt: 'latest' });
  await saver.flush();
  expect((await repo.get(value.id))!.originalPrompt).toBe('latest');
  expect(status).toHaveBeenLastCalledWith('saved');
  saver.dispose();
  const broken = { ...repo, save: vi.fn().mockRejectedValue(new Error('Quota exceeded')) },
    failed = vi.fn();
  const bad = createAutosave(broken, failed);
  bad.enqueue(value);
  await expect(bad.flush()).rejects.toThrow('Quota');
  expect(failed.mock.calls.some((c) => c[0] === 'saved')).toBe(false);
  expect(failed).toHaveBeenLastCalledWith('failed', 'Quota exceeded');
  bad.enqueue({ ...value, originalPrompt: 'Changed after failure' });
  expect(failed).toHaveBeenLastCalledWith('failed', 'Quota exceeded');
  bad.dispose();
});
it('reports conflicts instead of silently overwriting another tab', async () => {
  const repo = createExplorationRepository('conflict-' + crypto.randomUUID()),
    initial = await repo.save(makeExploration(), 0);
  await repo.save({ ...initial, originalPrompt: 'other tab' }, initial.revision);
  const status = vi.fn(),
    saver = createAutosave(repo, status);
  saver.enqueue({ ...initial, originalPrompt: 'stale' });
  await expect(saver.flush()).rejects.toThrow(/another tab/i);
  expect(status).toHaveBeenLastCalledWith('conflict', expect.any(String));
  expect((await repo.get(initial.id))!.originalPrompt).toBe('other tab');
  saver.dispose();
});
