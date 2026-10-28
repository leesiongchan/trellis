import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vite-plus/test';
import {
  ResearchRequestSchema,
  ResearchResultSchema,
  ApiErrorSchema
} from '../../src/lib/knowledge/schemas';

describe('shared Rust/browser contracts', () => {
  for (const [name, schema] of [
    ['requests', ResearchRequestSchema],
    ['results', ResearchResultSchema],
    ['errors', ApiErrorSchema]
  ] as const) {
    const cases = JSON.parse(
      readFileSync(
        new URL('../../../tests/fixtures/contracts/' + name + '.json', import.meta.url),
        'utf8'
      )
    );
    for (const sample of cases)
      it(name + ': ' + sample.name, () => {
        const parsed = schema.safeParse(sample.input);
        expect(parsed.success).toBe(sample.valid);
        if (sample.valid && sample.normalized) expect(parsed.data).toEqual(sample.normalized);
      });
  }
});
