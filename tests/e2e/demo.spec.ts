import { expect, test } from '@playwright/test';
import { mockResearch } from './helpers';

test('demo branches, citations and saved questions work without research requests', async ({
  page
}) => {
  const requests: string[] = [];
  page.on('request', (request) => {
    if (new URL(request.url()).pathname === '/api/research') requests.push(request.url());
  });
  await page.route('**/api/research', (route) => route.abort());
  await page.goto('/');
  await page.getByRole('button', { name: 'Try the microSD demo', exact: true }).click();
  await expect(page.getByRole('note', { name: 'Demo content' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Refresh research', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Expand NAND flash', exact: true }).click();
  await page.getByRole('button', { name: 'Expand Memory cells', exact: true }).click();
  await page.getByRole('button', { name: 'Read Quantum tunneling', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Quantum tunneling', exact: true })).toBeVisible();
  await expect(
    page.getByRole('link', {
      name: 'The Shining Sun and the Flash Memory for Storage',
      exact: true
    })
  ).toHaveAttribute('href', 'https://www.kioxia.com/en-jp/rd/movie.html');
  await expect(
    page.getByText('Prewritten demo · Sources for further reading.', { exact: true })
  ).toBeVisible();
  await page.getByRole('button', { name: 'Collapse branches', exact: true }).click();
  await page.getByRole('button', { name: 'Reveal in tree', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Read Quantum tunneling', exact: true })
  ).toBeVisible();
  await page.getByRole('button', { name: 'Return to root', exact: true }).click();
  await page.getByLabel('Your branch question').fill('Why do cameras sometimes lose a photo?');
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Why do cameras sometimes lose a photo?', exact: true })
  ).toBeVisible();
  await expect(page.getByText('Your question is saved.', { exact: false })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Retry research', exact: true })).toHaveCount(0);
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  await page.reload();
  await page.getByRole('button', { name: 'Resume microSD cards (demo)', exact: true }).click();
  await expect(page.getByRole('note', { name: 'Demo content' })).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Why do cameras sometimes lose a photo?', exact: true })
  ).toBeVisible();
  expect(requests).toHaveLength(0);

  // An explicit new research exploration keeps the original behavior; this response is mocked.
  await mockResearch(page);
  await page.getByRole('button', { name: 'Branch home', exact: true }).click();
  await page.getByLabel('What would you like to understand?').fill('How cities grow');
  await page.getByRole('button', { name: 'Start exploring', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'How cities grow', exact: true })).toBeVisible();
  await expect(page.getByRole('note', { name: 'Demo content' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Refresh research', exact: true })).toBeVisible();
  expect(requests).toHaveLength(1);
});

test('phone demo preserves local questions when switching map and reader', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const requests: string[] = [];
  page.on('request', (request) => {
    if (new URL(request.url()).pathname === '/api/research') requests.push(request.url());
  });
  await page.route('**/api/research', (route) => route.abort());
  await page.goto('/');
  await page.getByRole('button', { name: 'Try the microSD demo', exact: true }).click();
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  expect(
    await page
      .getByRole('button', { name: 'Your explorations', exact: true })
      .evaluate((element) => element.getBoundingClientRect().right)
  ).toBeLessThanOrEqual(382);
  await page.getByRole('button', { name: 'Read', exact: true }).click();
  await page.getByLabel('Your branch question').fill('A question from my phone');
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'A question from my phone', exact: true })
  ).toBeVisible();
  await page.getByRole('button', { name: 'Map', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Read A question from my phone', exact: true })
  ).toBeVisible();
  await page.getByRole('button', { name: 'Read', exact: true }).click();
  await expect(page.getByText('Your question is saved.', { exact: false })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  expect(requests).toHaveLength(0);
});
