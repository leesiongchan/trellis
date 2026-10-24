import { expect, test } from '@playwright/test';
import { mockResearch, startExample } from './helpers';
import { makeContent } from '../fixtures/knowledge';
test('a failed refresh preserves the prior explanation and explored branches', async ({ page }) => {
  await mockResearch(page);
  await startExample(page);
  await page.getByRole('button', { name: 'Expand NAND flash', exact: true }).click();
  await page.route('**/api/research', (route) =>
    route.fulfill({
      status: 502,
      json: {
        error: {
          code: 'UPSTREAM_FAILURE',
          message: 'The provider is unavailable.',
          retryable: true,
          requestId: 'test-request'
        }
      }
    })
  );
  await page.getByRole('button', { name: 'Refresh research' }).click();
  await expect(page.getByText('The provider is unavailable.')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'MicroSD reference', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Read Memory cells', exact: true })).toBeVisible();
});
test('model HTML remains inert text', async ({ page }) => {
  const content = makeContent();
  content.summary[0].text = '<img data-injected src=x onerror="window.__injected=true">';
  await page.route('**/api/research', (route) =>
    route.fulfill({ json: { kind: 'topic', content } })
  );
  await startExample(page);
  await expect(page.getByText(content.summary[0].text, { exact: true })).toBeVisible();
  expect(await page.locator('[data-injected]').count()).toBe(0);
});
test('storage failure is visible and does not falsely report saved', async ({ page }) => {
  await page.addInitScript(() => {
    IDBObjectStore.prototype.put = function () {
      throw new DOMException('Browser storage is full.', 'QuotaExceededError');
    };
  });
  await mockResearch(page);
  await startExample(page);
  await expect(page.getByText('Browser storage is full.', { exact: true })).toBeVisible();
  await expect(page.getByText('Not saved', { exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
});
test('out-of-order branch replies stay on their own nodes', async ({ page }) => {
  await mockResearch(page);
  await startExample(page);
  let resolveA!: () => Promise<void>;
  let requestA = false;
  await page.route('**/api/research', async (route) => {
    const { question } = route.request().postDataJSON();
    if (question === 'A') {
      requestA = true;
      await new Promise<void>((resolve) => {
        resolveA = async () => {
          await route.fulfill({
            json: { kind: 'topic', content: makeContent({ title: 'A explained', branches: [] }) }
          });
          resolve();
        };
      });
    } else
      await route.fulfill({
        json: { kind: 'topic', content: makeContent({ title: 'B explained', branches: [] }) }
      });
  });
  await page.getByLabel('Your branch question').fill('A');
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  await expect.poll(() => requestA).toBe(true);
  await page.getByRole('button', { name: 'Return to root' }).click();
  await page.getByLabel('Your branch question').fill('B');
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'B explained', exact: true })).toBeVisible();
  await resolveA();
  await expect(page.getByRole('heading', { name: 'B explained', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Read A', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'A explained', exact: true })).toBeVisible();
});
test('a second tab cannot silently overwrite an exploration', async ({ page, context }) => {
  await mockResearch(page);
  await startExample(page);
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  const other = await context.newPage();
  await mockResearch(other);
  await other.goto('/');
  await other.getByRole('button', { name: 'Resume microSD cards', exact: true }).click();
  await other.getByRole('button', { name: 'Read NAND flash', exact: true }).click();
  await expect(other.getByText('All changes saved', { exact: true })).toBeVisible();
  await expect(page.getByText('Changed in another tab', { exact: true })).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Reload saved version', exact: true })
  ).toBeVisible();
  await page.evaluate(() => {
    IDBObjectStore.prototype.get = function () {
      throw new Error('Read unavailable');
    };
  });
  await page.getByRole('button', { name: 'Reload saved version', exact: true }).click();
  await expect(
    page.getByText('The saved version could not be read. Your current work is still open.', {
      exact: false
    })
  ).toBeVisible();
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  await other.close();
});
test('deleting an exploration during research does not resurrect it', async ({ page }) => {
  await mockResearch(page);
  await startExample(page);
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  let pending = false;
  await page.route('**/api/research', async (route) => {
    pending = true;
    await new Promise((r) => setTimeout(r, 800));
    await route
      .fulfill({ json: { kind: 'topic', content: makeContent({ title: 'Late result' }) } })
      .catch(() => {});
  });
  await page.getByLabel('Your branch question').fill('A pending question');
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  await expect.poll(() => pending).toBe(true);
  await page.getByRole('button', { name: 'Your explorations', exact: true }).click();
  await page.getByRole('button', { name: 'Delete microSD cards', exact: true }).click();
  await page.getByRole('button', { name: 'Delete exploration', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'A little curiosity. An entire world.', exact: true })
  ).toBeVisible();
  await expect(page.getByRole('button', { name: 'Resume microSD cards', exact: true })).toHaveCount(
    0
  );
  await page.reload();
  await expect(page.getByRole('button', { name: 'Resume microSD cards', exact: true })).toHaveCount(
    0
  );
});
test('reloading a deletion from another tab discards late results without recreating the tree', async ({
  page,
  context
}) => {
  await mockResearch(page);
  await startExample(page);
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  const other = await context.newPage();
  await other.goto('/');
  let release!: () => void;
  let pending = false;
  await page.route('**/api/research', async (route) => {
    pending = true;
    await new Promise<void>((resolve) => {
      release = resolve;
    });
    await route
      .fulfill({ json: { kind: 'topic', content: makeContent({ title: 'Late result' }) } })
      .catch(() => {});
  });
  await page.getByLabel('Your branch question').fill('A pending question');
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  await expect.poll(() => pending).toBe(true);
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  await other.reload();
  await other.getByRole('button', { name: 'Delete microSD cards', exact: true }).click();
  await other.getByRole('button', { name: 'Delete exploration', exact: true }).click();
  await page.getByRole('button', { name: 'Reload saved version', exact: true }).click();
  release();
  await expect(
    page.getByRole('heading', { name: 'A little curiosity. An entire world.', exact: true })
  ).toBeVisible();
  await expect(
    page.getByText('This exploration was deleted in another tab.', { exact: false })
  ).toBeVisible();
  await page.reload();
  await expect(page.getByRole('button', { name: 'Resume microSD cards', exact: true })).toHaveCount(
    0
  );
  await other.close();
});
test('clarification keeps the original question and continues the same exploration', async ({
  page
}) => {
  let count = 0;
  await page.route('**/api/research', (route) => {
    count++;
    const input = route.request().postDataJSON();
    return route.fulfill({
      json: input.clarification
        ? { kind: 'topic', content: makeContent({ title: 'Java the programming language' }) }
        : { kind: 'clarification', question: 'Which Java do you mean?' }
    });
  });
  await page.goto('/');
  await page.getByLabel('What would you like to understand?').fill('Java');
  await page.getByRole('button', { name: 'Start exploring', exact: true }).click();
  await page.getByLabel('Which Java do you mean?').fill('The programming language');
  await page.getByRole('button', { name: 'Continue exploring ↗', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Java the programming language', exact: true })
  ).toBeVisible();
  expect(count).toBe(2);
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('button', { name: 'Resume Java', exact: true })).toBeVisible();
});
