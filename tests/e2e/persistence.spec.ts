import { expect, test } from './fixtures';
import { observeResearch, startExample } from './helpers';
import { makeExploration } from '../fixtures/knowledge';

test('switching saved explorations restores each exact pan and zoom without research', async ({
  page
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
  await page.evaluate(
    async (items) => {
      const database = await new Promise<IDBDatabase>((resolve, reject) => {
        const request = indexedDB.open('branch-knowledge-v1', 1);
        request.onupgradeneeded = () =>
          request.result.createObjectStore('explorations', { keyPath: 'id' });
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
      });
      const transaction = database.transaction('explorations', 'readwrite');
      for (const item of items) transaction.objectStore('explorations').put(item);
      await new Promise<void>((resolve, reject) => {
        transaction.oncomplete = () => resolve();
        transaction.onerror = () => reject(transaction.error);
      });
      database.close();
    },
    [first, second]
  );
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
