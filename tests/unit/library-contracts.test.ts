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
