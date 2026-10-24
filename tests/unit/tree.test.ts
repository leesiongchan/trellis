import { expect, it } from 'vitest';
import { layoutTree, visibleTree } from '../../src/lib/knowledge/tree';
import { makeExploration } from '../fixtures/knowledge';
it('lays out a deep tree without recursion and hides collapsed descendants', () => {
  const e = makeExploration();
  let parent = 'root';
  for (let i = 0; i < 200; i++) {
    const id = 'n' + i;
    e.nodes[id] = { ...e.nodes.root, id, parentId: parent, childIds: [] };
    e.nodes[parent].childIds = [id];
    e.view.expandedNodeIds.push(id);
    parent = id;
  }
  expect(visibleTree(e).nodes.length).toBe(201);
  const positions = layoutTree(e);
  expect(positions.at(-1)!.x).toBe(200 * 320);
  e.view.expandedNodeIds = [];
  expect(visibleTree(e).nodes.length).toBe(1);
});
