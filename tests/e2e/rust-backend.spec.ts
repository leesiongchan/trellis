import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
const cookieFixture = JSON.parse(
  readFileSync(new URL('../fixtures/contracts/cookies.json', import.meta.url), 'utf8')
)[0];
test('real Rust research reaches local provider fixtures through generation and grounding', async ({
  page,
  baseURL
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  const requests: string[] = [];
  page.on('request', (request) => {
    if (new URL(request.url()).pathname === '/api/research')
      requests.push(request.postData() ?? '');
  });
  await page.goto('/access?key=' + encodeURIComponent(cookieFixture.key));
  await expect(page).toHaveURL(baseURL + '/');
  await page.getByLabel('What would you like to understand?').fill('microSD cards');
  await page.getByRole('button', { name: 'Start exploring', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Expand NAND flash', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
  expect(requests).toHaveLength(2);
  const metrics = await page.request.get('/metrics');
  expect(metrics.status()).toBe(200);
  const text = await metrics.text();
  for (const stage of ['search', 'generation', 'grounding', 'verification'])
    expect(text).toMatch(
      new RegExp('branch_research_stages_total\\{stage="' + stage + '",outcome="ok"\\} 2')
    );
  const rejected = await page.request.post('/api/research', {
    headers: { origin: 'https://hostile.example', 'x-forwarded-host': new URL(baseURL!).host },
    data: {}
  });
  expect(rejected.status()).toBe(403);
  const wrongHost = await page.request.post('/api/research', {
    headers: { origin: baseURL!, host: 'hostile.example' },
    data: {}
  });
  expect(wrongHost.status()).toBe(403);
  expect(errors).toEqual([]);
});
