import type { Exploration } from './types';
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
export function legacyIdentity(value: Exploration) {
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
