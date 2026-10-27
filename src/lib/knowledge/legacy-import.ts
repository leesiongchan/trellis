import { ExplorationSchema } from './schemas';
import { canonicalLegacyJson, legacyIdentity } from './legacy-identity';
export { canonicalLegacyJson } from './legacy-identity';
const MAX_BYTES = 32 * 1024 * 1024;
const encoder = new TextEncoder();
export async function legacyDigest(input: unknown): Promise<string> {
  const value = ExplorationSchema.parse(input);
  if (value.mode === 'demo') throw new Error('Demo explorations stay on this device.');
  const digest = await crypto.subtle.digest(
    'SHA-256',
    encoder.encode(canonicalLegacyJson(legacyIdentity(value)))
  );
  return Array.from(new Uint8Array(digest), (v) => v.toString(16).padStart(2, '0')).join('');
}
export async function encodeLegacyImport(input: unknown) {
  const value = ExplorationSchema.parse(input);
  const bytes = encoder.encode(JSON.stringify(value));
  const expectedNodes = Object.keys(value.nodes).length;
  if (bytes.length > MAX_BYTES || expectedNodes > 2000)
    throw new Error(
      'This exploration exceeds the import limit. Keep and export its original browser copy.'
    );
  const batches: string[][] = [];
  const decoder = new TextDecoder('utf-8', { fatal: true });
  for (let offset = 0; offset < bytes.length;) {
    let end = Math.min(offset + 256 * 1024, bytes.length);
    while (end < bytes.length && (bytes[end] & 0xc0) === 0x80) end--;
    batches.push([decoder.decode(bytes.subarray(offset, end))]);
    offset = end;
  }
  return {
    legacyExplorationId: value.id,
    digest: await legacyDigest(value),
    expectedBytes: bytes.length,
    expectedNodes,
    batches
  };
}
