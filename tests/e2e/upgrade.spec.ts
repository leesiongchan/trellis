import { expect, test } from './fixtures';
import { readFileSync } from 'node:fs';
const legacy = JSON.parse(
  readFileSync(new URL('../fixtures/contracts/legacy-exploration.json', import.meta.url), 'utf8')
);
const cookie = JSON.parse(
  readFileSync(new URL('../fixtures/contracts/cookies.json', import.meta.url), 'utf8')
)[0];
test('pre-Rust browser data and Node-signed cookies survive the same-origin upgrade', async ({
  page,
  context,
  baseURL
}) => {
  const requests: string[] = [];
  const errors: string[] = [];
  page.on('request', (request) => {
    if (/\/api\/appearances\/[^/]+\/research$/.test(new URL(request.url()).pathname)) requests.push(request.url());
  });
  page.on('pageerror', (error) => errors.push(error.message));
  await context.addCookies([
    {
      name: 'branch_access',
      value: cookie.token,
      url: baseURL!,
      httpOnly: true,
      sameSite: 'Strict',
      secure: baseURL!.startsWith('https:')
    }
  ]);
  await page.goto('/');
  await expect(page.getByLabel('What would you like to understand?')).toBeEnabled();
  await page.evaluate(async (value) => {
    const database = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('branch-knowledge-v1', 1);
      request.onupgradeneeded = () =>
        request.result.createObjectStore('explorations', { keyPath: 'id' });
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    const transaction = database.transaction('explorations', 'readwrite');
    transaction.objectStore('explorations').put(value);
    await new Promise<void>((resolve, reject) => {
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
    });
    database.close();
  }, legacy);
  await page.reload();
  await page.getByRole('button', { name: 'Resume microSD cards', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  const viewport = await page.locator('.svelte-flow__viewport').evaluate((element) => {
    const matrix = new DOMMatrixReadOnly(getComputedStyle(element).transform);
    return { x: matrix.e, y: matrix.f, zoom: matrix.a };
  });
  expect(viewport).toEqual(legacy.view.viewport);
  expect((await page.request.get('/metrics')).status()).toBe(200);
  expect(requests).toHaveLength(0);
  expect(errors).toEqual([]);
  if (baseURL!.startsWith('https:'))
    expect((await context.cookies()).find((value) => value.name === 'branch_access')?.secure).toBe(
      true
    );
});
