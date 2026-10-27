import { expect, test } from './fixtures';
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
    if (/\/api\/appearances\/[^/]+\/research$/.test(new URL(request.url()).pathname))
      requests.push(request.postData() ?? '');
  });
  await page.goto('/access?key=' + encodeURIComponent(cookieFixture.key));
  await expect(page).toHaveURL(baseURL + '/');
  const beforeMetrics = await (await page.request.get('/metrics')).text();
  const stageCount = (text: string, stage: string) =>
    Number(
      text.match(
        new RegExp('branch_research_stages_total\\{stage="' + stage + '",outcome="ok"\\} ([0-9]+)')
      )?.[1] ?? 0
    );
  const question = 'Rust fixture microSD ' + crypto.randomUUID();
  await page.getByLabel('What would you like to understand?').fill(question);
  await page.getByRole('button', { name: 'Start exploring', exact: true }).click();
  await expect(page.getByRole('heading', { name: question, exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Expand NAND flash', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
  await expect.poll(() => requests.length).toBe(2);
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  const metrics = await page.request.get('/metrics');
  expect(metrics.status()).toBe(200);
  const text = await metrics.text();
  for (const stage of ['search', 'generation', 'grounding', 'verification'])
    expect(stageCount(text, stage) - stageCount(beforeMetrics, stage), stage).toBe(2);
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
