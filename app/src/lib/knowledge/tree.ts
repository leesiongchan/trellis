import type { WorkspaceExploration, TopicNode } from './types';

export function visibleTree(value: WorkspaceExploration) {
  const expanded = new Set(value.view.expandedNodeIds),
    nodes: TopicNode[] = [],
    edges: { id: string; source: string; target: string }[] = [];
  const stack = [value.rootId],
    seen = new Set<string>();
  while (stack.length) {
    const id = stack.pop()!,
      node = value.nodes[id];
    if (!node || seen.has(id)) continue;
    seen.add(id);
    nodes.push(node);
    if (expanded.has(id))
      for (const child of [...node.childIds].reverse()) {
        if (value.nodes[child]) {
          edges.push({ id: id + ':' + child, source: id, target: child });
          stack.push(child);
        }
      }
  }
  return { nodes, edges };
}
export function layoutTree(value: WorkspaceExploration): { id: string; x: number; y: number }[] {
  const { nodes } = visibleTree(value),
    visible = new Set(nodes.map((n) => n.id)),
    positions = new Map<string, { id: string; x: number; y: number }>();
  const depths = new Map([[value.rootId, 0]]);
  for (const node of nodes)
    for (const child of node.childIds) depths.set(child, (depths.get(node.id) ?? 0) + 1);
  let leafY = 0;
  for (const node of [...nodes].reverse()) {
    const children = node.childIds.filter((id) => visible.has(id)).map((id) => positions.get(id)!);
    const y = children.length ? (children[0].y + children.at(-1)!.y) / 2 : leafY;
    if (!children.length) leafY += 174;
    positions.set(node.id, { id: node.id, x: (depths.get(node.id) ?? 0) * 320, y });
  }
  // Reverse traversal places earlier siblings lower; mirror to preserve their display order.
  return nodes.map((n) => {
    const p = positions.get(n.id)!;
    return { ...p, y: Math.max(0, leafY - 174) - p.y };
  });
}
export function ancestorPath(value: WorkspaceExploration, nodeId: string): TopicNode[] {
  const nodes: TopicNode[] = [],
    seen = new Set<string>();
  let node = value.nodes[nodeId];
  while (node && !seen.has(node.id)) {
    nodes.unshift(node);
    seen.add(node.id);
    if (!node.parentId) break;
    node = value.nodes[node.parentId];
  }
  return nodes;
}
