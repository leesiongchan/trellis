import { expect, test } from './fixtures';
import { observeResearch, startExample, seedServerExploration, authorize } from './helpers';
import { makeExploration } from '../fixtures/knowledge';

test('switching saved explorations restores each exact pan and zoom without research', async ({
  page,
  browser
}) => {
  const requests = await observeResearch(page);
  await page.goto('/');
  await expect(
    page.getByRole('heading', { name: 'A little curiosity. An entire world.', exact: true })
  ).toBeVisible();
  const first = makeExploration({ id: 'first', originalPrompt: 'First exploration', revision: 1 });
  const second = makeExploration({
    id: 'second',
    originalPrompt: 'Second exploration',
    revision: 1
  });
  first.view.viewport = { x: 120, y: 80, zoom: 0.65 };
  second.view.viewport = { x: -45, y: 150, zoom: 0.85 };
  await seedServerExploration(page, first);
  await seedServerExploration(page, second);
  await page.reload();
  const viewport = () =>
    page.locator('.svelte-flow__viewport').evaluate((element) => {
      const matrix = new DOMMatrixReadOnly(getComputedStyle(element).transform);
      return { x: matrix.e, y: matrix.f, zoom: matrix.a };
    });
  await page.getByRole('button', { name: 'Resume First exploration', exact: true }).click();
  await expect.poll(viewport).toEqual({ x: 120, y: 80, zoom: 0.65 });
  await page.getByRole('button', { name: 'Your explorations', exact: true }).click();
  await page.getByRole('button', { name: 'Resume Second exploration', exact: true }).click();
  await expect.poll(viewport).toEqual({ x: -45, y: 150, zoom: 0.85 });
  await page.getByRole('button', { name: 'Your explorations', exact: true }).click();
  await page.getByRole('button', { name: 'Resume First exploration', exact: true }).click();
  await expect.poll(viewport).toEqual({ x: 120, y: 80, zoom: 0.65 });
  const otherDevice = await browser.newContext({ baseURL: new URL(page.url()).origin });
  try {
    const other = await otherDevice.newPage();
    await authorize(other);
    await other.getByRole('button', { name: 'Resume First exploration', exact: true }).click();
    await expect
      .poll(() =>
        other.locator('.svelte-flow__viewport').evaluate((element) => {
          const matrix = new DOMMatrixReadOnly(getComputedStyle(element).transform);
          return { x: matrix.e, y: matrix.f, zoom: matrix.a };
        })
      )
      .toEqual({ x: 120, y: 80, zoom: 0.65 });
  } finally {
    await otherDevice.close();
  }
  expect(requests).toHaveLength(0);
});
test('reloads saved content, expansion, selection and viewport without new research', async ({
  page
}) => {
  const requests = await observeResearch(page);
  await startExample(page);
  await page.getByRole('button', { name: 'Expand NAND flash', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Expand Memory cells', exact: true })
  ).toBeVisible();
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  const before = requests.length;
  await page.reload();
  await page.getByRole('button', { name: 'Resume microSD cards', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Expand Memory cells', exact: true })
  ).toBeVisible();
  await expect(page.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
  expect(requests.length).toBe(before);
});
