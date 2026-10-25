// Fixed, non-secret compatibility data. Run before removing the Node backend.
import { createHmac } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import { makeRequest, makeContent, makeExploration } from '../tests/fixtures/knowledge.ts';

const output = new URL('../tests/fixtures/contracts/', import.meta.url);
await mkdir(output, { recursive: true });
const request = makeRequest();
const { clarification: _missing, ...withoutClarification } = request;
const requests = [
  { name: 'ordinary question', input: request, valid: true, normalized: request },
  {
    name: 'trim and strip extras',
    input: { ...request, question: '\ufeff microSD cards \u00a0', extra: true },
    valid: true,
    normalized: request
  },
  {
    name: 'explicit null',
    input: { ...request, clarification: null },
    valid: true,
    normalized: request
  },
  { name: 'missing nullable field', input: withoutClarification, valid: false },
  { name: 'blank', input: { ...request, question: ' \t\n\ufeff' }, valid: false },
  { name: 'oversized', input: { ...request, question: 'x'.repeat(501) }, valid: false },
  {
    name: 'astral boundary',
    input: { ...request, question: '😀'.repeat(500) },
    valid: true,
    normalized: { ...request, question: '😀'.repeat(500) }
  },
  { name: 'astral over boundary', input: { ...request, question: '😀'.repeat(501) }, valid: false },
  {
    name: 'NEL is not ECMAScript trim',
    input: { ...request, question: '\u0085' },
    valid: true,
    normalized: { ...request, question: '\u0085' }
  },
  {
    name: 'ancestor total boundary',
    input: { ...request, ancestors: [{ title: '', summary: 'x'.repeat(6000) }] },
    valid: true
  },
  {
    name: 'ancestor total overflow',
    input: {
      ...request,
      ancestors: [
        { title: '', summary: 'x'.repeat(3000) },
        { title: '', summary: 'x'.repeat(3001) }
      ]
    },
    valid: false
  },
  {
    name: 'ancestor astral total boundary',
    input: { ...request, ancestors: [{ title: '', summary: '😀'.repeat(3000) }] },
    valid: true
  },
  {
    name: 'ancestor astral total overflow',
    input: { ...request, ancestors: [{ title: '', summary: '😀'.repeat(3001) }] },
    valid: false
  },
  {
    name: 'too many ancestors',
    input: {
      ...request,
      ancestors: Array.from({ length: 51 }, () => ({ title: '', summary: '' }))
    },
    valid: false
  }
];
const content = makeContent();
const results = [
  { name: 'complete topic', input: { kind: 'topic', content }, valid: true },
  {
    name: 'clarification',
    input: { kind: 'clarification', question: ' Which Java? ' },
    valid: true,
    normalized: { kind: 'clarification', question: 'Which Java?' }
  },
  {
    name: 'insufficient evidence',
    input: { kind: 'insufficient-evidence', message: 'No evidence.' },
    valid: true
  },
  {
    name: 'missing sources',
    input: { kind: 'topic', content: { ...content, sources: [] } },
    valid: false
  },
  { name: 'unknown result', input: { kind: 'invented' }, valid: false },
  {
    name: 'invalid timestamp',
    input: { kind: 'topic', content: { ...content, researchedAt: 'yesterday' } },
    valid: false
  },
  {
    name: 'unsafe source',
    input: {
      kind: 'topic',
      content: {
        ...content,
        sources: [{ ...content.sources[0], url: 'https://127.0.0.1/private' }]
      }
    },
    valid: false
  },
  {
    name: 'missing required analogy',
    input: {
      kind: 'topic',
      content: Object.fromEntries(Object.entries(content).filter(([key]) => key !== 'analogy'))
    },
    valid: false
  }
];
// Legacy protocol from dc1f24a:src/lib/server/access.ts, using a fake key
// and a frozen clock; no user cookie or secret is read by this script.
const key = 'fixture-access-key-'.repeat(4);
const expires = '4102444800000';
const token =
  expires +
  '.' +
  createHmac('sha256', key)
    .update('branch-session-v1:' + expires)
    .digest('base64url');
const files = {
  requests,
  results,
  errors: [
    {
      name: 'access error',
      input: {
        code: 'ACCESS_REQUIRED',
        message: 'Open your private access link to enable research.',
        retryable: false,
        requestId: 'fixture-request'
      },
      valid: true
    }
  ],
  cookies: [{ key, expiresMs: Number(expires), nowMs: 1792944000000, token }],
  'legacy-exploration': makeExploration({ id: 'pre-rust-exploration', revision: 1 })
};
for (const [name, value] of Object.entries(files))
  await writeFile(new URL(name + '.json', output), JSON.stringify(value, null, 2) + '\n');
