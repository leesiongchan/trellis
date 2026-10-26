import { ExplorationSchema } from './schemas';
import type { Exploration } from './types';
const MAX_BYTES = 32 * 1024 * 1024;
const encoder = new TextEncoder();
function compareCodePoints(a: string, b: string): number {
  let i = 0,
    j = 0;
  while (i < a.length && j < b.length) {
    const x = a.codePointAt(i)!,
      y = b.codePointAt(j)!;
    if (x !== y) return x - y;
    i += x > 0xffff ? 2 : 1;
    j += y > 0xffff ? 2 : 1;
  }
  return (i < a.length ? 1 : 0) - (j < b.length ? 1 : 0);
}
// Write sorted keys directly: rebuilding an object reorders integer-like keys in JS.
export function canonicalLegacyJson(value: unknown): string {
  if (value === null || typeof value === 'string' || typeof value === 'boolean')
    return JSON.stringify(value);
  if (typeof value === 'number' && Number.isSafeInteger(value)) return JSON.stringify(value);
  if (Array.isArray(value)) return '[' + value.map(canonicalLegacyJson).join(',') + ']';
  if (typeof value === 'object' && value) {
    const record = value as Record<string, unknown>;
    return (
      '{' +
      Object.keys(record)
        .sort(compareCodePoints)
        .map((k) => JSON.stringify(k) + ':' + canonicalLegacyJson(record[k]))
        .join(',') +
      '}'
    );
  }
  throw new Error('The saved exploration cannot be encoded. Keep and export its original copy.');
}
function identity(value: Exploration) {
  const nodes = Object.fromEntries(
    Object.entries(value.nodes).map(([id, n]) => [
      id,
      {
        id: n.id,
        parentId: n.parentId,
        question: n.question,
        relationship: n.relationship,
        teaser: n.teaser,
        content: n.content,
        childIds: n.childIds
      }
    ])
  );
  return {
    digestVersion: 1,
    id: value.id,
    originalPrompt: value.originalPrompt,
    rootId: value.rootId,
    nodes,
    createdAt: value.createdAt
  };
}
export async function legacyDigest(input: unknown): Promise<string> {
  const value = ExplorationSchema.parse(input);
  if (value.mode === 'demo') throw new Error('Demo explorations stay on this device.');
  const digest = await crypto.subtle.digest(
    'SHA-256',
    encoder.encode(canonicalLegacyJson(identity(value)))
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
