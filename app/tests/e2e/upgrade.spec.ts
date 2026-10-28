import { expect, test } from './fixtures';
import { readFileSync } from 'node:fs';
import { seedServerExploration } from './helpers';
const legacy = JSON.parse(
  readFileSync(
    new URL('../../../tests/fixtures/contracts/legacy-exploration.json', import.meta.url),
    'utf8'
  )
);
const cookie = JSON.parse(
  readFileSync(new URL('../../../tests/fixtures/contracts/cookies.json', import.meta.url), 'utf8')
)[0];
test('server data and Node-signed cookies survive the same-origin upgrade', async ({
  page,
  context,
  baseURL
}) => {
  const requests: string[] = [];
  const errors: string[] = [];
  page.on('request', (request) => {
    if (/\/api\/appearances\/[^/]+\/research$/.test(new URL(request.url()).pathname))
      requests.push(request.url());
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
  await seedServerExploration(page, legacy);
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
