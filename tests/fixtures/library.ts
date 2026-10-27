import { makeExploration } from './knowledge';
import type {
  ExplorationSnapshot,
  LibraryInfo,
  OperationView
} from '../../src/lib/knowledge/library-types';
export const library: LibraryInfo = {
  protocolVersion: 2,
  schemaVersion: 2,
  libraryId: 'library-a',
  libraryEpoch: 1,
  discovery: 'unavailable'
};
export function snapshot(): ExplorationSnapshot {
  const e = makeExploration();
  return {
    schemaVersion: 2,
    libraryEpoch: 1,
    exploration: {
      id: 'e1',
      originalPrompt: e.originalPrompt,
      rootId: 'root',
      revision: 0,
      createdAt: e.createdAt,
      updatedAt: e.updatedAt
    },
    appearances: [
      {
        id: 'root',
        explorationId: 'e1',
        parentId: null,
        topicId: 'topic1',
        question: e.originalPrompt,
        locale: 'en',
        clarification: null,
        ancestors: [],
        interpretation: 'literal',
        relationship: 'Starting topic',
        reason: null,
        reasonSources: [],
        reasonAnswerVersionId: null,
        answerVersionId: 'answer1',
        childIds: [],
        teaser: ''
      }
    ],
    answers: [
      {
        id: 'answer1',
        topicId: 'topic1',
        revision: 1,
        origin: 'research',
        provenance: { kind: 'unavailable' },
        content: e.nodes.root.content!,
        createdAt: e.createdAt
      }
    ]
  };
}
export const completed = (id: string): OperationView => ({
  id,
  state: 'completed',
  result: { kind: 'exploration', explorationId: 'e1', appearanceId: 'root', revision: 0 },
  error: null,
  createdAt: '2026-10-27T00:00:00Z',
  updatedAt: '2026-10-27T00:00:00Z'
});
export const jsonResponse = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { 'X-Branch-Protocol': '2', 'Content-Type': 'application/json' }
  });
