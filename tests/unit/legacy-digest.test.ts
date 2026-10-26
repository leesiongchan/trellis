import { expect, it } from 'vitest';
import fixture from '../fixtures/library/migration.json';
import { legacyDigest, encodeLegacyImport } from '../../src/lib/knowledge/legacy-import';
import { ExplorationSchema } from '../../src/lib/knowledge/schemas';
it('preserves v1 Unicode and ignores only device state in content identity', async () => {
  const a = structuredClone(fixture);
  a.nodes.root.question = '😀'.repeat(500);
  expect(ExplorationSchema.safeParse(a).success).toBe(true);
  const d = await legacyDigest(a);
  a.view.viewport.x = 10.5;
  a.revision = 99;
  a.updatedAt = '2026-10-26T00:00:00.000Z';
  a.nodes.root.isRefreshing = true;
  expect(await legacyDigest(a)).toBe(d);
  a.nodes.root.content.summary[0].text = 'A changed saved answer';
  expect(await legacyDigest(a)).not.toBe(d);
});
it('uses a shared versioned canonical digest and bounded JSON fragment batches', async () => {
  expect(await legacyDigest(fixture)).toBe(
    '9508538a67a945ddecd250143319ab11f85dc9150ae54b4205d0fc2fdec68132'
  );
  const encoded = await encodeLegacyImport(fixture);
  expect(encoded.digest).toBe(await legacyDigest(fixture));
  expect(
    encoded.batches.every(
      (b) => new TextEncoder().encode(JSON.stringify(b)).length <= 2 * 1024 * 1024
    )
  ).toBe(true);
  expect(JSON.parse(encoded.batches.flat().join(''))).toEqual(ExplorationSchema.parse(fixture));
});
it('sorts integer-like and supplementary Unicode keys without object re-enumeration', async () => {
  const { canonicalLegacyJson } = await import('../../src/lib/knowledge/legacy-import');
  expect(canonicalLegacyJson({ '2': false, '10': true, '𐀀': null, '': [] })).toBe(
    '{"10":true,"2":false,"":[],"𐀀":null}'
  );
});
