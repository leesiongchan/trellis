import { expect, test } from './fixtures';
import { observeResearch, startExample } from './helpers';

test('readable map cards keep separate selection and expansion actions', async ({ page }) => {
  await observeResearch(page);
  await startExample(page);
  const read = page.getByRole('button', { name: 'Read NAND flash', exact: true });
  const title = read.locator('strong');
  expect(
    await title.evaluate((element) => parseFloat(getComputedStyle(element).fontSize))
  ).toBeGreaterThanOrEqual(18);
  expect(
    await read
      .locator('[data-topic-label]')
      .evaluate((element) => parseFloat(getComputedStyle(element).fontSize))
  ).toBeGreaterThanOrEqual(14);
  const expand = page.getByRole('button', { name: 'Expand NAND flash', exact: true });
  expect(
    await expand.evaluate((element) => parseFloat(getComputedStyle(element).minHeight))
  ).toBeGreaterThanOrEqual(44);
  expect(
    await read.evaluate((element) => {
      const style = getComputedStyle(element.parentElement!);
      return [style.width, style.height];
    })
  ).toEqual(['240px', '150px']);
  await expect(
    page.getByRole('main', { name: 'Exploration workspace', exact: true })
  ).toBeVisible();
  await read.click();
  await expect(
    page.getByRole('heading', { name: 'NAND flash', level: 1, exact: true })
  ).toBeVisible();
  await expect(expand).toHaveAttribute('aria-expanded', 'false');
});

test('reduced-motion map controls settle without animation or new research', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const requests = await observeResearch(page);
  await startExample(page);
  const before = requests.length;
  for (const name of ['Zoom in', 'Zoom out', 'Fit entire tree', 'Refocus selected topic']) {
    await page.getByRole('button', { name, exact: true }).click();
    const settled = await page.evaluate(async () => {
      const viewport = document.querySelector('.svelte-flow__viewport')!;
      await new Promise(requestAnimationFrame);
      const first = getComputedStyle(viewport).transform;
      await new Promise((resolve) => setTimeout(resolve, 250));
      return first === getComputedStyle(viewport).transform;
    });
    expect(settled, name).toBe(true);
  }
  expect(requests.length).toBe(before);
});

test('enlarged phone text keeps the reader usable and continuation cards inside it', async ({
  page
}) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await observeResearch(page);
  await startExample(page);
  await page.getByRole('button', { name: 'Read', exact: true }).click();
  await page.evaluate(() => (document.documentElement.style.fontSize = '200%'));
  const main = page.getByRole('main', { name: 'Exploration workspace', exact: true });
  expect((await main.boundingBox())!.height).toBeGreaterThanOrEqual(240);
  const reader = page.getByRole('complementary', { name: 'Topic explanation', exact: true });
  expect(await reader.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(true);
  const branch = reader.getByRole('button', {
    name: 'NAND flash Flash retains data without power.',
    exact: true
  });
  await branch.scrollIntoViewIfNeeded();
  const bounds = (await branch.boundingBox())!;
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(320);
  const pageWidth = await page.evaluate(() => ({
    width: document.documentElement.scrollWidth,
    buttons: [...document.querySelectorAll('header button')].map((element) => ({
      label: element.textContent?.trim(),
      right: element.getBoundingClientRect().right
    }))
  }));
  expect(pageWidth.width, JSON.stringify(pageWidth)).toBeLessThanOrEqual(320);
});
