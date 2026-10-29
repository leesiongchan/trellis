import { expect, test } from './fixtures';
import { observeResearch, startExample, seedServerExploration } from './helpers';
import { makeExploration } from '../fixtures/knowledge';

test('a lost refresh preserves the prior answer and recovers its original intent', async ({
  page
}) => {
  await observeResearch(page);
  await startExample(page);
  await page.getByRole('button', { name: 'Expand NAND flash', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Read Memory cells', exact: true })).toBeVisible();
  const ids: string[] = [];
  await page.route('**/api/appearances/*/research', (route) => {
    ids.push(route.request().postDataJSON().operationId);
    return route.abort();
  });
  await page.getByRole('button', { name: 'Refresh research' }).click();
  await expect(
    page.getByText('Server disconnected. Reconnect to load or change saved content.', {
      exact: false
    })
  ).toBeVisible();
  await expect(page.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'MicroSD reference', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Read Memory cells', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Refresh research' })).toBeDisabled();
  await page.unroute('**/api/appearances/*/research');
  page.on('request', (request) => {
    if (/\/appearances\/[^/]+\/research$/.test(new URL(request.url()).pathname))
      ids.push(request.postDataJSON().operationId);
  });
  await page.getByRole('button', { name: 'Reconnect library' }).click();
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  expect(ids).toHaveLength(2);
  expect(ids[1]).toBe(ids[0]);
});

test('saved model HTML remains inert text from the server', async ({ page }) => {
  await page.goto('/');
  const legacy = makeExploration({
    id: 'hostile-' + crypto.randomUUID(),
    originalPrompt: 'Hostile model text'
  });
  const hostile = '<img data-injected src=x onerror="window.__injected=true">';
  legacy.nodes.root.content!.summary[0].text = hostile;
  await seedServerExploration(page, legacy);
  await page.reload();
  await page.getByRole('button', { name: 'Resume Hostile model text', exact: true }).click();
  await expect(
    page
      .getByRole('complementary', { name: 'Topic explanation', exact: true })
      .getByText(hostile, { exact: false })
  ).toBeVisible();
  expect(await page.locator('[data-injected]').count()).toBe(0);
});

test('research works with browser persistence unavailable and no demo controls', async ({
  page
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, 'indexedDB', {
      get() {
        throw new Error('Browser storage unavailable');
      }
    });
  });
  await observeResearch(page);
  await startExample(page);
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Your explorations', exact: true }).click();
  await expect(
    page.getByRole('button', { name: /demo|browser copies|browser originals/i })
  ).toHaveCount(0);
});

test('a late acknowledged branch snapshot preserves the current selection', async ({ page }) => {
  await observeResearch(page);
  await startExample(page);
  let release!: () => void;
  let held = false;
  await page.route('**/api/explorations/*', async (route) => {
    if (route.request().method() !== 'GET') return route.continue();
    const response = await route.fetch();
    const snapshot = await response.json();
    if (
      !held &&
      snapshot.appearances?.some(
        (a: { question: string; answerVersionId: string | null }) =>
          a.question === 'A' && a.answerVersionId
      )
    ) {
      held = true;
      await new Promise<void>((resolve) => {
        release = resolve;
      });
    }
    await route.fulfill({ response });
  });
  await page.getByLabel('Your branch question').fill('A');
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  await expect.poll(() => held).toBe(true);
  await page.getByRole('button', { name: 'Return to root' }).click();
  release();
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  await page.getByLabel('Your branch question').fill('B');
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'B', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Read A', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'A', exact: true })).toBeVisible();
});

test('a second tab cannot overwrite an exploration and failed reload preserves the current answer', async ({
  page,
  context
}) => {
  await observeResearch(page);
  await startExample(page);
  const other = await context.newPage();
  await other.goto('/');
  await other.getByRole('button', { name: 'Resume microSD cards', exact: true }).click();
  await other.getByRole('button', { name: 'Read NAND flash', exact: true }).click();
  await expect(other.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
  await expect(page.getByText('Changed in another tab', { exact: true })).toBeVisible();
  await page.route('**/api/explorations/*', (route) => route.abort());
  await page.getByRole('button', { name: 'Reload saved version', exact: true }).click();
  await expect(
    page.getByText('The saved version could not be read. Your current work is still open.', {
      exact: false
    })
  ).toBeVisible();
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  await other.close();
});

test('deletion from another tab fences pending work without recreating the tree', async ({
  page,
  context
}) => {
  await observeResearch(page);
  await startExample(page);
  const other = await context.newPage();
  await other.goto('/');
  let held = false,
    operationId = '',
    release!: () => void;
  await page.route('**/api/appearances/*/research', async (route) => {
    const response = await route.fetch();
    operationId = (await response.json()).id;
    held = true;
    await new Promise<void>((resolve) => {
      release = resolve;
    });
    await route.fulfill({ response }).catch(() => {});
  });
  await page.getByLabel('Your branch question').fill('A pending question');
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  await expect.poll(() => held).toBe(true);
  await expect
    .poll(
      async () =>
        (
          await (
            await page.request.get('/api/operations/' + operationId, {
              headers: { 'X-Branch-Protocol': '2' }
            })
          ).json()
        ).state
    )
    .not.toBe('pending');
  await other.reload();
  const drawer = other.getByRole('region', { name: 'Your explorations drawer', exact: true });
  if (!(await drawer.isVisible()))
    await other.getByRole('button', { name: 'Your explorations', exact: true }).click();
  await drawer.getByRole('button', { name: 'Delete microSD cards', exact: true }).click();
  await other.getByRole('button', { name: 'Delete exploration', exact: true }).click();
  await expect(
    other.getByRole('button', { name: 'Resume microSD cards', exact: true })
  ).toHaveCount(0);
  release();
  const reload = page.getByRole('button', { name: 'Reload saved version', exact: true });
  if (await reload.isVisible()) await reload.click();
  await expect(
    page.getByText('This exploration was deleted in another tab.', { exact: false })
  ).toBeVisible();
  await page.reload();
  await expect(page.getByRole('button', { name: 'Resume microSD cards', exact: true })).toHaveCount(
    0
  );
  await other.close();
});

test('reload resolves an admitted lost response without another research submission', async ({
  page
}) => {
  const requests = await observeResearch(page);
  await startExample(page);
  let lost = false;
  await page.route('**/api/appearances/*/research', async (route) => {
    await route.fetch();
    lost = true;
    await route.abort();
  });
  await page.getByRole('button', { name: 'Refresh research' }).click();
  await expect.poll(() => lost).toBe(true);
  await expect(page.getByRole('button', { name: 'Reconnect library' })).toBeVisible();
  const before = requests.length;
  await page.unroute('**/api/appearances/*/research');
  await page.reload();
  await page.getByRole('button', { name: 'Trellis home', exact: true }).click();
  await page.getByRole('button', { name: 'Resume microSD cards', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  expect(requests.length).toBe(before);
});

test('clarification keeps the original question and continues the same exploration', async ({
  page
}) => {
  const requests = await observeResearch(page);
  await page.goto('/');
  await page.getByLabel('What would you like to understand?').fill('Java');
  await page.getByRole('button', { name: 'Start exploring', exact: true }).click();
  await page.getByLabel('Which Java do you mean?').fill('The programming language');
  await page.getByRole('button', { name: 'Continue exploring ↗', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Java the programming language', exact: true })
  ).toBeVisible();
  expect(requests).toHaveLength(2);
  await page.reload();
  await expect(page.getByRole('button', { name: 'Resume Java', exact: true })).toBeVisible();
});

test('a reload cannot restore saved content while the server is unreachable', async ({ page }) => {
  await observeResearch(page);
  await startExample(page);
  await page.route('**/api/**', (route) => route.abort());
  await page.reload();
  await expect(page.getByRole('button', { name: 'Resume microSD cards', exact: true })).toHaveCount(
    0
  );
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toHaveCount(0);
  await expect(page.getByText('Cannot reach the server.', { exact: false }).first()).toBeVisible();
});
