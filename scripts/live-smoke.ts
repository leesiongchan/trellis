import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { z } from 'zod';

const origin = process.env.APP_ORIGIN ?? 'http://127.0.0.1:3000';
if (!process.env.OPENAI_API_KEY || !process.env.TAVILY_API_KEY || !process.env.APP_ACCESS_KEY) {
  console.error(
    'Live research not verified: configure OPENAI_API_KEY, TAVILY_API_KEY and APP_ACCESS_KEY in .env, then restart the server.'
  );
  process.exit(2);
}
const access = await fetch(
  new URL('/access?key=' + encodeURIComponent(process.env.APP_ACCESS_KEY), origin),
  { redirect: 'manual', signal: AbortSignal.timeout(15000) }
);
assert.equal(access.status, 303, 'Private access failed.');
const cookie = access.headers
  .getSetCookie()
  .map((c) => c.split(';')[0])
  .join('; ');
assert.ok(cookie, 'The server did not establish a session.');
const Content = z.object({
  title: z.string().min(1),
  summary: z.array(z.object({ text: z.string().min(1) })).min(1),
  sections: z
    .array(
      z.object({
        paragraphs: z
          .array(
            z.object({
              text: z.string().min(1),
              evidence: z.array(z.object({ sourceId: z.string(), quote: z.string().min(1) })).min(1)
            })
          )
          .min(1)
      })
    )
    .min(1),
  sources: z.array(z.object({ id: z.string(), url: z.url() })).min(1),
  branches: z.array(z.object({ question: z.string().min(1) })).min(1)
});
const explorationId = randomUUID();
async function research(question: string, ancestors: { title: string; summary: string }[] = []) {
  const response = await fetch(new URL('/api/research', origin), {
    method: 'POST',
    headers: { 'content-type': 'application/json', origin, cookie },
    body: JSON.stringify({
      explorationId,
      nodeId: randomUUID(),
      question,
      clarification: null,
      ancestors
    }),
    signal: AbortSignal.timeout(125000)
  });
  if (!response.ok)
    throw new Error(
      'Live research failed with HTTP ' + response.status + '. Check the server by its request ID.'
    );
  const result = await response.json();
  assert.equal(result.kind, 'topic', 'Research did not return a complete explanation.');
  const content = Content.parse(result.content),
    sourceIds = new Set(content.sources.map((s) => s.id));
  for (const section of content.sections)
    for (const paragraph of section.paragraphs)
      for (const ref of paragraph.evidence)
        assert.ok(sourceIds.has(ref.sourceId), 'An evidence reference had no source.');
  assert.ok(
    content.sections.flatMap((s) => s.paragraphs).reduce((n, p) => n + p.text.length, 0) > 400,
    'The explanation is too short to verify detail.'
  );
  return content;
}
try {
  const root = await research('How does a microSD card store data?');
  await research(root.branches[0].question, [
    {
      title: root.title,
      summary: root.summary
        .map((p) => p.text)
        .join(' ')
        .slice(0, 400)
    }
  ]);
  console.log(
    'Live smoke passed: two researched topics, detailed explanations, valid source references and further branches.'
  );
} catch (error) {
  console.error(error instanceof Error ? error.message : 'Live smoke failed.');
  process.exitCode = 1;
}
