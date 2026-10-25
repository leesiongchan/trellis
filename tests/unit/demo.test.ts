import { expect, it, vi } from 'vitest';
import { createDemoExploration } from '../../src/lib/knowledge/demo';
import { ExplorationSchema } from '../../src/lib/knowledge/schemas';
import { createSession } from '../../src/lib/knowledge/session.svelte';
import { makeContent, makeExploration } from '../fixtures/knowledge';

it('creates independent valid demo trees while legacy saves default to research', () => {
  const first = createDemoExploration(),
    second = createDemoExploration();
  expect(ExplorationSchema.parse(first).mode).toBe('demo');
  expect(first.id).not.toBe(second.id);
  expect(Object.keys(first.nodes)).toHaveLength(8);
  expect(first.view.expandedNodeIds).toEqual([first.rootId]);
  first.nodes[first.rootId].question = 'changed';
  expect(second.nodes[second.rootId].question).toBe('microSD cards');
  const { mode: _mode, ...legacy } = makeExploration();
  expect(ExplorationSchema.parse(legacy).mode).toBe('research');
});

it('keeps every demo action local and preserves unanswered custom questions', async () => {
  const research = vi.fn(async () => ({ kind: 'topic' as const, content: makeContent() })),
    changes = vi.fn();
  const session = createSession({ research, onChange: changes });
  session.load(createDemoExploration());
  const root = session.exploration!.rootId;
  const nand = session.exploration!.nodes[root].childIds[0];
  await session.expand(nand);
  await session.refresh(nand);
  await session.clarify(nand, 'a different answer');
  const custom = await session.addQuestion(root, 'How does a camera choose where to write?');
  expect(session.exploration!.nodes[custom].content).toBeNull();
  expect(session.exploration!.nodes[custom].status).toBe('suggested');
  expect(session.exploration!.view.selectedNodeId).toBe(custom);
  expect(await session.addQuestion(root, 'How does a camera choose where to write?')).toBe(custom);
  expect(research).not.toHaveBeenCalled();
  expect(changes).toHaveBeenCalled();
  await session.start('A real research topic');
  expect(research).toHaveBeenCalledOnce();
  expect(session.exploration!.mode).toBe('research');
});
