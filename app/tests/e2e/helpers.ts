import { expect, type Page } from '@playwright/test';
export const fixtureKey = 'fixture-access-key-'.repeat(4);
export async function authorize(page: Page) {
  await page.goto('/access?key=' + encodeURIComponent(fixtureKey));
}
// Observe genuine Rust requests and saved snapshots; all answers pass through embedded storage.
export async function observeResearch(page: Page, authenticate = true) {
  if (authenticate) await authorize(page);
  const questions = new Map<string, string>(),
    requests: string[] = [];
  page.on('response', async (response) => {
    if (/\/api\/explorations\/[^/]+$/.test(new URL(response.url()).pathname) && response.ok()) {
      try {
        const snapshot = await response.json();
        for (const a of snapshot.appearances ?? []) {
          questions.set(a.id, a.question);
          for (let i = 0; i < requests.length; i++)
            if (requests[i] === a.id) requests[i] = a.question;
        }
      } catch {
        /* disconnected response */
      }
    }
  });
  page.on('request', (request) => {
    if (/\/api\/appearances\/[^/]+\/research$/.test(new URL(request.url()).pathname)) {
      const input = request.postDataJSON();
      requests.push(questions.get(input.payload.appearanceId) ?? input.payload.appearanceId);
    }
  });
  return requests;
}
export async function startExample(page: Page) {
  await page.goto('/');
  await page.getByLabel('What would you like to understand?').fill('microSD cards');
  await page.getByRole('button', { name: 'Start exploring', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'microSD cards', exact: true, includeHidden: true })
  ).toBeAttached();
  await expect(
    page.getByRole('button', { name: 'Refresh research', exact: true, includeHidden: true })
  ).toBeAttached();
}

// Seed the real server import API; browser storage is never used by fixtures.
export async function seedServerExploration(
  page: Page,
  value: import('../../src/lib/knowledge/types').Exploration
) {
  const { createHash } = await import('node:crypto');
  const { canonicalLegacyJson, legacyIdentity } =
    await import('../../../scripts/legacy-identity.ts');
  const info = await (
    await page.request.get('/api/library', { headers: { 'X-Branch-Protocol': '2' } })
  ).json();
  const headers = {
    'X-Branch-Protocol': '2',
    'X-Branch-Library': info.libraryId,
    origin: new URL(page.url()).origin
  };
  const command = (payload: unknown) => ({
    operationId: crypto.randomUUID(),
    libraryEpoch: info.libraryEpoch,
    expectedRevision: null,
    payload
  });
  const send = async (path: string, payload: unknown) => {
    const response = await page.request.post(path, { headers, data: command(payload) });
    expect(response.ok(), await response.text()).toBe(true);
    return response.json();
  };
  const start = await send('/api/imports', {
    legacyExplorationId: value.id,
    contentDigest: createHash('sha256')
      .update(canonicalLegacyJson(legacyIdentity(value)))
      .digest('hex'),
    expectedBytes: Buffer.byteLength(JSON.stringify(value)),
    expectedNodes: Object.keys(value.nodes).length,
    explicitRestore: false
  });
  await send(`/api/imports/${start.id}/batches`, {
    importId: start.id,
    batchIndex: 0,
    records: [JSON.stringify(value)]
  });
  const done = await send(`/api/imports/${start.id}/completion`, { importId: start.id });
  const view = {
    ...value.view,
    selectedNodeId: done.mapping[value.view.selectedNodeId],
    expandedNodeIds: value.view.expandedNodeIds.map((id) => done.mapping[id])
  };
  const response = await page.request.put(`/api/explorations/${done.explorationId}/view`, {
    headers,
    data: command({ explorationId: done.explorationId, view })
  });
  expect(response.ok(), await response.text()).toBe(true);
  return done;
}
