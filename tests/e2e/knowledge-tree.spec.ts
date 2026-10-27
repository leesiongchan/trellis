import { expect, test } from './fixtures';
import { observeResearch, startExample } from './helpers';
test('opens three branch levels, reads sources and reuses completed content', async ({ page }) => {
  const requests = await observeResearch(page);
  await startExample(page);
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'MicroSD reference', exact: true })).toHaveAttribute(
    'href',
    'https://example.org/memory'
  );
  await page.getByRole('button', { name: 'Expand NAND flash', exact: true }).click();
  await page.getByRole('button', { name: 'Expand Memory cells', exact: true }).click();
  await page.getByRole('button', { name: 'Read Quantum tunneling', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Quantum tunneling', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Refresh research', exact: true })).toBeVisible();
  await expect(page.getByText('All changes saved', { exact: true })).toBeVisible();
  const before = requests.length;
  await page.getByRole('button', { name: 'Return to root', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  expect(requests.length).toBe(before);
});
test('selects without expanding and allows a custom question without duplicate siblings', async ({
  page
}) => {
  const requests = await observeResearch(page);
  await startExample(page);
  await page.getByRole('button', { name: 'Read NAND flash', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Expand Memory cells', exact: true })).toHaveCount(
    0
  );
  await page.getByLabel('Your branch question').fill('Why does memory wear out?');
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Why does memory wear out?', exact: true })
  ).toBeVisible();
  await expect.poll(() => requests.includes('Why does memory wear out?')).toBe(true);
});
