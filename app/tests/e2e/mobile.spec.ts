import { expect, test } from './fixtures';
import { observeResearch, startExample } from './helpers';
for (const width of [320, 390]) {
  test(`phone switches between reader and map without losing the selected branch at ${width}px`, async ({
    page
  }) => {
    await page.setViewportSize({ width, height: 844 });
    await observeResearch(page);
    await startExample(page);
    await page.getByRole('button', { name: 'Map', exact: true }).click();
    await page.getByRole('button', { name: 'Read NAND flash', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Map', exact: true }).click();
    await expect(page.getByRole('region', { name: 'Topic map', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Expand NAND flash', exact: true }).click();
    await expect(
      page.getByRole('button', { name: 'Read Memory cells', exact: true })
    ).toBeVisible();
    await page.getByRole('button', { name: 'Read', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
    const inactiveNode = page.getByRole('button', { name: 'Read NAND flash', exact: true });
    expect(await inactiveNode.evaluate((element) => element.closest('[inert]') !== null)).toBe(
      true
    );
    await inactiveNode.evaluate((element) => (element as HTMLButtonElement).focus());
    await expect(inactiveNode).not.toBeFocused();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true
    );
  });
}
test('keyboard can select, expand and refocus topics', async ({ page }) => {
  await observeResearch(page);
  await startExample(page);
  const expand = page.getByRole('button', { name: 'Expand NAND flash', exact: true });
  await expand.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', { name: 'Read Memory cells', exact: true })).toBeVisible();
  const refocus = page.getByRole('button', { name: 'Refocus selected topic' });
  await refocus.focus();
  await page.keyboard.press('Enter');
  await expect(refocus).toBeFocused();
});
