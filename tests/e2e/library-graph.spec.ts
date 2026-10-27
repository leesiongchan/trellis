import { expect, test } from './fixtures';
import { fixtureKey, observeResearch, seedLegacy } from './helpers';
import { makeExploration } from '../fixtures/knowledge';
import { PURGE_CONFIRMATION } from '../../src/lib/knowledge/library-schemas';
import type { APIRequestContext, Page } from '@playwright/test';
const question = 'How does flash retain data?';
type Seed = {
  sourceExplorationId: string;
  sourceAppearanceId: string;
  sourceTopicId: string;
  sourceAnswerVersionId: string;
  targetExplorationId: string;
  targetTopicId: string;
};
const headers = { 'X-Branch-Protocol': '2' };
async function seed(page: Page, origin: string): Promise<Seed> {
  const response = await page.request.post('/__fixtures/library-graph', { headers: { origin } });
  expect(response.ok(), await response.text()).toBe(true);
  return response.json();
}
async function openSource(page: Page) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Your explorations', exact: true }).click();
  await page
    .getByRole('button', { name: 'Resume ' + question, exact: true })
    .first()
    .click();
  await expect(page.getByRole('heading', { name: question, exact: true })).toBeVisible();
}
async function read(request: APIRequestContext, path: string) {
  const response = await request.get(path, { headers });
  expect(response.ok()).toBe(true);
  return response.json();
}
async function command(
  request: APIRequestContext,
  origin: string,
  path: string,
  payload: unknown,
  revision: number | null = null
) {
  const info = await read(request, '/api/library');
  const response = await request.post(path, {
    headers: { ...headers, origin, 'X-Branch-Library': info.libraryId },
    data: {
      operationId: crypto.randomUUID(),
      libraryEpoch: info.libraryEpoch,
      expectedRevision: revision,
      payload
    }
  });
  expect(response.ok(), await response.text()).toBe(true);
  return response.json();
}

test('saved connections paginate, follow and return without research, and dismissal survives another device', async ({
  page,
  browser,
  baseURL
}) => {
  const data = await seed(page, baseURL!);
  const requests = await observeResearch(page);
  await openSource(page);
  const region = page.getByRole('region', { name: 'Topic connections', exact: true });
  await expect(region.getByRole('article')).toHaveCount(5);
  await expect(region.getByRole('button', { name: 'More connections', exact: true })).toBeVisible();
  await region.getByRole('button', { name: 'More connections', exact: true }).click();
  await expect.poll(() => region.getByRole('article').count()).toBeGreaterThan(5);
  for (
    let i = 0;
    i < 20 && (await region.getByRole('button', { name: 'More connections', exact: true }).count());
    i++
  ) {
    if (
      (await region
        .getByRole('article', { name: 'Equivalent question connection', exact: true })
        .count()) &&
      (await region
        .getByRole('article', { name: 'Overlapping answer connection', exact: true })
        .count())
    )
      break;
    await region.getByRole('button', { name: 'More connections', exact: true }).click();
  }
  const equivalent = region
    .getByRole('article', { name: 'Equivalent question connection', exact: true })
    .first();
  await expect(equivalent).toBeVisible();
  await expect(
    region.getByRole('article', { name: 'Overlapping answer connection', exact: true }).first()
  ).toBeVisible();
  await expect(
    equivalent.getByText(/Saved public fixture evidence connects these topics/)
  ).toBeVisible();
  expect(await page.locator('[data-connection-injected]').count()).toBe(0);
  const before = requests.length;
  await equivalent.getByRole('button', { name: 'Open saved topic', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'How can flash retain data?', exact: true })
  ).toBeVisible();
  await page.getByRole('button', { name: 'Back to previous topic', exact: true }).click();
  await expect(page.getByRole('heading', { name: question, exact: true })).toBeVisible();
  const researchRequestsBeforeFollow = before,
    researchRequestsAfterFollow = requests.length;
  expect(researchRequestsAfterFollow).toBe(researchRequestsBeforeFollow);
  const candidate = region
    .getByRole('article', { name: 'Similar question connection', exact: true })
    .first();
  const connectionId = await candidate.getAttribute('data-connection-id');
  await candidate.getByRole('button', { name: 'Dismiss connection', exact: true }).click();
  await expect(page.locator('[data-connection-id="' + connectionId + '"]')).toHaveCount(0);
  await page.reload();
  await openSource(page);
  await expect(page.locator('[data-connection-id="' + connectionId + '"]')).toHaveCount(0);
  const device = await browser.newContext();
  try {
    const other = await device.newPage();
    await other.goto(baseURL + '/access?key=' + encodeURIComponent(fixtureKey));
    await other.getByRole('button', { name: 'Your explorations', exact: true }).click();
    await other
      .getByRole('button', { name: 'Resume ' + question, exact: true })
      .first()
      .click();
    await expect(other.getByRole('region', { name: 'Topic connections' })).toBeVisible();
    await expect(other.locator('[data-connection-id="' + connectionId + '"]')).toHaveCount(0);
    expect((await read(other.request, baseURL + '/api/library')).libraryId).toBe(
      (await read(page.request, '/api/library')).libraryId
    );
  } finally {
    await device.close();
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole('button', { name: 'Read', exact: true }).click();
  const clipped = await page.locator('header button').evaluateAll((buttons) =>
    buttons
      .filter((button) => {
        const r = button.getBoundingClientRect();
        return r.width > 0 && (r.right > innerWidth || r.left < 0);
      })
      .map((button) => button.getAttribute('aria-label') || button.textContent?.trim())
  );
  expect(clipped).toEqual([]);
  await page.getByRole('button', { name: 'Saved topics', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('region', { name: 'Saved topics', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Close saved topics', exact: true }).click();
  expect(data.sourceTopicId).toBeTruthy();
});

test('another appearance refresh preserves the original citations until explicit version choice', async ({
  page,
  browser,
  baseURL
}) => {
  const data = await seed(page, baseURL!);
  await openSource(page);
  const original = await read(page.request, '/api/explorations/' + data.sourceExplorationId);
  const originalAnswerSourcesBeforeRefresh = original.answers.find(
    (a: { id: string }) => a.id === data.sourceAnswerVersionId
  ).content.sources;
  await command(page.request, baseURL!, '/api/topics/' + data.sourceTopicId + '/explorations', {
    topicId: data.sourceTopicId,
    answerVersionId: data.sourceAnswerVersionId
  });
  const device = await browser.newContext();
  try {
    const other = await device.newPage();
    await other.goto(baseURL + '/access?key=' + encodeURIComponent(fixtureKey));
    await other.getByRole('button', { name: 'Your explorations', exact: true }).click();
    await other
      .getByRole('button', { name: 'Resume ' + question, exact: true })
      .first()
      .click();
    await other.getByRole('button', { name: 'Refresh research', exact: true }).click();
    await expect(other.getByText('All changes saved', { exact: true })).toBeVisible();
    await expect(page.getByText('A newer saved answer is available.', { exact: true })).toBeVisible(
      { timeout: 10000 }
    );
    const retained = await read(page.request, '/api/explorations/' + data.sourceExplorationId);
    const originalAnswerSourcesAfterRefresh = retained.answers.find(
      (a: { id: string }) => a.id === data.sourceAnswerVersionId
    ).content.sources;
    expect(originalAnswerSourcesAfterRefresh).toEqual(originalAnswerSourcesBeforeRefresh);
    const topic = await read(page.request, '/api/topics/' + data.sourceTopicId);
    expect(topic.currentAnswerVersionId).not.toBe(data.sourceAnswerVersionId);
    await page
      .getByRole('button', { name: 'Use version ' + topic.selectedAnswer.revision, exact: true })
      .click();
    await expect
      .poll(
        async () =>
          (
            await read(page.request, '/api/explorations/' + data.sourceExplorationId)
          ).appearances.find((a: { id: string }) => a.id === data.sourceAppearanceId)
            .answerVersionId
      )
      .toBe(topic.currentAnswerVersionId);
    await expect(page.getByText('A newer saved answer is available.', { exact: true })).toHaveCount(
      0
    );
  } finally {
    await device.close();
  }
});

test('changed browser copies have clear provenance and preserve both saved answers', async ({
  page
}) => {
  await page.goto('/');
  const original = makeExploration({
    id: 'graph-legacy-' + crypto.randomUUID(),
    originalPrompt: 'Preserved browser copy'
  });
  await seedLegacy(page, [original]);
  await page.reload();
  await expect(page.getByText('Imported browser copy', { exact: true })).toBeVisible();
  original.nodes.root.content!.summary[0].text = 'An edited original snapshot';
  await seedLegacy(page, [original]);
  await page.reload();
  await expect(page.getByText('Changed browser copy', { exact: true })).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Resume Preserved browser copy', exact: true })
  ).toHaveCount(2);
});

test('explicit restore renews a deleted imported copy on the same browser without duplicate copies or research', async ({
  page
}) => {
  const requests = await observeResearch(page);
  const prompt = 'Restorable browser copy ' + crypto.randomUUID();
  const original = makeExploration({
    id: 'restore-' + crypto.randomUUID(),
    originalPrompt: prompt
  });
  await seedLegacy(page, [original]);
  await page.reload();
  await expect(
    page.getByRole('button', { name: 'Resume ' + prompt, exact: true }).first()
  ).toBeVisible();
  const first = (await read(page.request, '/api/explorations')).items.find(
    (e: { originalPrompt: string }) => e.originalPrompt === prompt
  );
  expect(first).toBeTruthy();
  await page.getByRole('button', { name: 'Your explorations', exact: true }).click();
  await page
    .getByRole('button', { name: 'Delete ' + prompt, exact: true })
    .first()
    .click();
  await page.getByRole('button', { name: 'Delete exploration', exact: true }).click();
  await expect
    .poll(async () =>
      (await read(page.request, '/api/explorations')).items.some(
        (e: { id: string }) => e.id === first.id
      )
    )
    .toBe(false);
  await page.getByRole('button', { name: 'Restore browser copies', exact: true }).click();
  await expect(page.getByText(/1 browser copies restored/)).toBeVisible();
  const restored = (await read(page.request, '/api/explorations')).items.filter(
    (e: { originalPrompt: string }) => e.originalPrompt === prompt
  );
  expect(restored).toHaveLength(1);
  expect(restored[0].id).not.toBe(first.id);
  const snapshot = await read(page.request, '/api/explorations/' + restored[0].id);
  expect(snapshot.answers[0].content).toEqual(original.nodes.root.content);
  expect(snapshot.answers[0].provenance.kind).toBe('legacy');
  await page.getByRole('button', { name: 'Restore browser copies', exact: true }).click();
  await expect(page.getByText(/0 browser copies restored/)).toBeVisible();
  expect(
    (await read(page.request, '/api/explorations')).items.filter(
      (e: { originalPrompt: string }) => e.originalPrompt === prompt
    )
  ).toHaveLength(1);
  expect(requests).toHaveLength(0);
});

test('retained topics seed compatible roots and confirmed purge fences offline originals', async ({
  page,
  baseURL
}) => {
  const data = await seed(page, baseURL!);
  const requests = await observeResearch(page);
  await openSource(page);
  await page.getByRole('button', { name: 'Your explorations', exact: true }).click();
  await page.getByRole('button', { name: 'Delete ' + question, exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Saved topics and answer history remain');
  await page.getByRole('button', { name: 'Delete exploration', exact: true }).click();
  await page.getByRole('button', { name: 'Saved topics', exact: true }).click();
  await page.getByLabel('Search saved topics').fill(question);
  await page.getByRole('button', { name: 'Read saved topic ' + question, exact: true }).click();
  await expect(page.getByText('Read-only saved topic', { exact: true })).toBeVisible();
  const before = requests.length;
  await page
    .getByRole('button', { name: 'Start exploration from this topic', exact: true })
    .click();
  await expect(page.getByRole('heading', { name: question, exact: true })).toBeVisible();
  expect(requests.length).toBe(before);
  const original = makeExploration({ id: 'offline-original-' + crypto.randomUUID() });
  const epoch = (await read(page.request, '/api/library')).libraryEpoch;
  await page.getByRole('button', { name: 'Your explorations', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Clear saved library', exact: true })
  ).toBeEnabled();
  await seedLegacy(page, [original]);
  await page.getByRole('button', { name: 'Clear saved library', exact: true }).click();
  await expect(
    page.getByText(/Offline browser originals and retained backups require separate clearing/)
  ).toBeVisible();
  await page.getByLabel('Library deletion confirmation').fill(PURGE_CONFIRMATION);
  await page
    .getByRole('button', { name: 'Delete all server library content', exact: true })
    .click();
  await expect(page.getByText('Server library cleared.', { exact: false })).toBeVisible();
  expect((await read(page.request, '/api/library')).libraryEpoch).toBeGreaterThan(epoch);
  await page.reload();
  await expect(page.getByText(/previously deleted or older-library copy/)).toBeVisible();
  expect((await read(page.request, '/api/explorations')).items).toHaveLength(0);
  const kept = await page.evaluate(async (id) => {
    const db = await new Promise<IDBDatabase>((resolve) => {
      const request = indexedDB.open('branch-knowledge-v1', 1);
      request.onsuccess = () => resolve(request.result);
    });
    const tx = db.transaction('explorations');
    const value = await new Promise((resolve) => {
      const request = tx.objectStore('explorations').get(id);
      request.onsuccess = () => resolve(request.result);
    });
    db.close();
    return value;
  }, original.id);
  expect(kept).toEqual(original);
  expect(data.sourceTopicId).toBeTruthy();
});
