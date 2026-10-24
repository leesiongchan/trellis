import type { Exploration, ResearchRequest, TopicContent } from '../../src/lib/knowledge/types';

export function makeContent(overrides: Partial<TopicContent> = {}): TopicContent {
  const p = (id: string, text: string) => ({ id, text, evidence: [{ sourceId: 's1', quote: 'Flash retains data without power.' }] });
  return {
    title: 'microSD cards', summary: [p('p1', 'Flash retains data without power.')],
    sections: [{ heading: 'A closer look', paragraphs: [p('p2', 'A controller manages flash storage.')] }],
    analogy: null, limitations: null,
    sources: [{ id: 's1', title: 'MicroSD reference', url: 'https://example.org/memory', publisher: 'Example', retrievedAt: '2026-10-25T00:00:00.000Z' }],
    branches: [{ title: 'NAND flash', question: 'NAND flash', relationship: 'Component', reason: p('b1', 'The memory inside the card.') }],
    researchedAt: '2026-10-25T00:00:00.000Z', ...overrides
  };
}
export function makeRequest(overrides: Partial<ResearchRequest> = {}): ResearchRequest {
  return { explorationId: 'e1', nodeId: 'root', question: 'microSD cards', clarification: null, ancestors: [], ...overrides };
}
export function makeExploration(overrides: Partial<Exploration> = {}): Exploration {
  return {
    schemaVersion: 1, id: 'e1', originalPrompt: 'microSD cards', rootId: 'root', revision: 0,
    nodes: { root: { id: 'root', parentId: null, question: 'microSD cards', relationship: 'Starting topic', teaser: '', status: 'ready', content: makeContent(), childIds: [], lastError: null, isRefreshing: false } },
    view: { selectedNodeId: 'root', expandedNodeIds: ['root'], viewport: { x: 0, y: 0, zoom: 1 }, mobilePane: 'map' },
    createdAt: '2026-10-25T00:00:00.000Z', updatedAt: '2026-10-25T00:00:00.000Z', ...overrides
  };
}
