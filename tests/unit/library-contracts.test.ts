import { expect, it } from 'vitest';
import cases from '../fixtures/library/contracts.json';
import {
  CreateExplorationCommandSchema,
  ResearchCommandSchema,
  LibraryInfoSchema
} from '../../src/lib/knowledge/library-schemas';
it('matches the shared Rust library contracts', () => {
  for (const sample of cases) {
    const schema = {
      create: CreateExplorationCommandSchema,
      research: ResearchCommandSchema,
      library: LibraryInfoSchema
    }[sample.kind]!;
    const result = schema.safeParse(sample.input);
    expect(result.success, sample.name).toBe(sample.valid);
    if ('normalized' in sample && result.success)
      expect(result.data, sample.name).toEqual(sample.normalized);
  }
});

it('answer contracts preserve immutable generation/input provenance and mark older records unavailable', async () => {
  const { AnswerVersionSchema } = await import('../../src/lib/knowledge/library-schemas');
  const { snapshot } = await import('../fixtures/library');
  const original = snapshot().answers[0];
  const provenance = {
    kind: 'research',
    attemptId: 'attempt-a',
    input: {
      explorationId: 'e1',
      nodeId: 'root',
      question: 'Power at 1 mW',
      clarification: null,
      ancestors: []
    },
    locale: 'en',
    interpretation: 'literal',
    contextEncodingVersion: 1,
    generation: {
      provider: 'openai-responses',
      requestedModel: 'requested-model',
      reportedModel: 'reported-model',
      promptSha256: 'a'.repeat(64),
      outputSchemaSha256: 'b'.repeat(64),
      reasoningEffort: 'low',
      maxOutputTokens: 8000,
      responseStorage: false
    }
  };
  expect(AnswerVersionSchema.parse({ ...original, provenance }).provenance).toEqual(provenance);
  expect(AnswerVersionSchema.parse({ ...original, provenance: undefined }).provenance).toEqual({
    kind: 'unavailable'
  });
});
