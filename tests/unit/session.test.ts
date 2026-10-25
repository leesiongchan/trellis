import { describe, expect, it, vi } from 'vitest';
import { createSession } from '../../src/lib/knowledge/session.svelte';
import { makeContent, makeExploration } from '../fixtures/knowledge';
import type { ResearchClient, ResearchResult } from '../../src/lib/knowledge/types';

const reply = (title = 'microSD cards'): ResearchResult => ({
  kind: 'topic',
  content: makeContent({ title })
});
function deferred<T>() {
  let resolve!: (value: T) => void, reject!: (reason: unknown) => void;
  const promise = new Promise<T>((a, b) => {
    resolve = a;
    reject = b;
  });
  return { promise, resolve, reject };
}
function setup(research: ResearchClient = vi.fn(async () => reply())) {
  return createSession({ research, onChange: vi.fn() });
}
describe('exploration session', () => {
  it('does not postpone autosave for duplicate viewport events or floating point noise', () => {
    const onChange = vi.fn();
    const session = createSession({ research: vi.fn(), onChange });
    session.load(makeExploration());
    session.setView({ viewport: { x: 0, y: 0, zoom: 1 } });
    session.setView({ viewport: { x: 0.00001, y: -0.00001, zoom: 1.00000001 } });
    expect(onChange).not.toHaveBeenCalled();
    session.setView({ viewport: { x: 20, y: 30, zoom: 0.8 } });
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(session.exploration!.view.viewport).toEqual({ x: 20, y: 30, zoom: 0.8 });
    session.setView({ viewport: { x: 20, y: 30, zoom: 0.8 }, mobilePane: 'reader' });
    expect(onChange).toHaveBeenCalledTimes(2);
    expect(session.exploration!.view.mobilePane).toBe('reader');
  });
  it('onlyResearchesOpenedNodes and selectDoesNotExpand', async () => {
    const research = vi.fn(async () => reply()),
      session = setup(research);
    await session.start('microSD cards');
    const e = session.exploration!,
      child = e.nodes[e.rootId].childIds[0];
    expect(research).toHaveBeenCalledTimes(1);
    expect(e.nodes[child].status).toBe('suggested');
    await session.select(child);
    expect(session.exploration!.view.expandedNodeIds).not.toContain(child);
    expect(research).toHaveBeenCalledTimes(2);
    await session.select(child);
    expect(research).toHaveBeenCalledTimes(2);
  });
  it('deduplicatesDoubleExpansion and collapsePreservesDescendants', async () => {
    const pending = deferred<ResearchResult>(),
      research = vi
        .fn<ResearchClient>()
        .mockResolvedValueOnce(reply())
        .mockReturnValue(pending.promise);
    const session = setup(research);
    await session.start('microSD cards');
    const child = session.exploration!.nodes[session.exploration!.rootId].childIds[0];
    const a = session.expand(child),
      b = session.expand(child);
    await Promise.resolve();
    expect(research).toHaveBeenCalledTimes(2);
    pending.resolve(reply('NAND flash'));
    await Promise.all([a, b]);
    const before = structuredClone(session.exploration!.nodes[child]);
    session.collapse(child);
    await session.expand(child);
    expect(session.exploration!.nodes[child]).toEqual(before);
    expect(research).toHaveBeenCalledTimes(2);
  });
  it('ignoresPreviousExplorationResults and disposePreventsDeletedExplorationResurrection', async () => {
    const old = deferred<ResearchResult>(),
      onChange = vi.fn();
    const session = createSession({ research: () => old.promise, onChange });
    const request = session.start('old');
    session.load(makeExploration());
    old.resolve(reply('wrong'));
    await request;
    expect(session.exploration!.originalPrompt).toBe('microSD cards');
    const other = deferred<ResearchResult>(),
      changes = vi.fn();
    const disposed = createSession({ research: () => other.promise, onChange: changes });
    const active = disposed.start('delete me');
    disposed.dispose();
    changes.mockClear();
    other.resolve(reply());
    await active;
    expect(changes).not.toHaveBeenCalled();
  });
  it('failedRefreshPreservesContentAndChildren', async () => {
    const research = vi
      .fn<ResearchClient>()
      .mockResolvedValueOnce(reply())
      .mockRejectedValueOnce(new Error('Offline'));
    const session = setup(research);
    await session.start('microSD cards');
    const root = session.exploration!.rootId,
      previous = structuredClone(session.exploration!.nodes[root]);
    await session.refresh(root);
    expect(session.exploration!.nodes[root].content).toEqual(previous.content);
    expect(session.exploration!.nodes[root].childIds).toEqual(previous.childIds);
    expect(session.exploration!.nodes[root].status).toBe('ready');
    expect(session.exploration!.nodes[root].lastError).toBeTruthy();
  });
  it('reusesExistingChildrenOnRetry and suppressesAncestorAndSiblingDuplicates', async () => {
    const session = setup();
    await session.start('microSD cards');
    const root = session.exploration!.rootId,
      child = session.exploration!.nodes[root].childIds[0];
    expect(await session.addQuestion(root, ' NAND FLASH? ')).toBe(child);
    await session.refresh(root);
    expect(session.exploration!.nodes[root].childIds).toEqual([child]);
    await expect(session.addQuestion(child, 'microSD cards')).rejects.toThrow(/already/i);
  });
  it('attaches out-of-order replies to their original nodes', async () => {
    const a = deferred<ResearchResult>(),
      b = deferred<ResearchResult>();
    const research = vi
      .fn<ResearchClient>()
      .mockResolvedValueOnce(reply())
      .mockImplementation((r) => (r.question === 'A' ? a.promise : b.promise));
    const session = setup(research);
    await session.start('microSD cards');
    const root = session.exploration!.rootId;
    const pa = session.addQuestion(root, 'A'),
      pb = session.addQuestion(root, 'B');
    b.resolve(reply('B explained'));
    await pb;
    a.resolve(reply('A explained'));
    await pa;
    const nodes = Object.values(session.exploration!.nodes);
    expect(nodes.find((n) => n.question === 'A')!.content!.title).toBe('A explained');
    expect(nodes.find((n) => n.question === 'B')!.content!.title).toBe('B explained');
  });
  it('boundsContextWithoutLimitingDepth and preserves originalPrompt on clarification', async () => {
    const research = vi
      .fn<ResearchClient>()
      .mockResolvedValueOnce({ kind: 'clarification', question: 'Which Java?' })
      .mockResolvedValue(reply('Java language'));
    const session = setup(research);
    await session.start('Java');
    await session.clarify(session.exploration!.rootId, 'programming language');
    expect(session.exploration!.originalPrompt).toBe('Java');
    expect(research.mock.calls[1][0].clarification).toBe('programming language');
    for (let i = 0; i < 20; i++)
      await session.addQuestion(session.exploration!.view.selectedNodeId, 'Topic ' + i);
    const input = research.mock.calls.at(-1)![0];
    expect(input.ancestors.reduce((n, a) => n + a.summary.length, 0)).toBeLessThanOrEqual(6000);
  });
});
