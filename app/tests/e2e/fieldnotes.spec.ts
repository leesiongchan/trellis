import { expect, test } from './fixtures';
import { observeResearch, seedServerExploration, startExample } from './helpers';
import { makeExploration } from '../fixtures/knowledge';

test('start and saved exploration controls reflow at 320px', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await observeResearch(page);
  const question = 'How can we understand the physical limits of storing information? '
    .repeat(6)
    .trim();
  const exploration = makeExploration({ id: crypto.randomUUID(), originalPrompt: question });
  exploration.nodes.root.question = question;
  await seedServerExploration(page, exploration);
  await page.reload();
  await expect(page.getByLabel('What would you like to understand?')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Start exploring', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.getByRole('button', { name: 'Your explorations', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Resume ' + question, exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Delete ' + question, exact: true }).click();
  await expect(page.getByRole('button', { name: 'Keep it', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Delete exploration', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('selected topic styles survive navigation and reduced-motion map controls', async ({
  page
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const requests = await observeResearch(page);
  await startExample(page);
  const selected = page.getByRole('button', { name: 'Read NAND flash', exact: true });
  await selected.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
  await expect(selected).toHaveAttribute('aria-current', 'location');
  const appearance = (element: HTMLElement | SVGElement) => {
    const style = getComputedStyle(element.parentElement!);
    return { border: style.borderColor, background: style.backgroundColor };
  };
  const root = page.getByRole('button', { name: 'Read microSD cards', exact: true });
  expect(await selected.evaluate(appearance)).not.toEqual(await root.evaluate(appearance));
  const refocus = page.getByRole('button', { name: 'Refocus selected topic', exact: true });
  await refocus.focus();
  await page.keyboard.press('Enter');
  await expect(refocus).toBeFocused();
  expect(
    await refocus.evaluate((element) => {
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return (
        style.outlineStyle !== 'none' &&
        parseFloat(style.outlineWidth) >= 2 &&
        rect.top >= 0 &&
        rect.bottom <= innerHeight
      );
    })
  ).toBe(true);
  const before = requests.length;
  await page.getByRole('button', { name: 'Return to root', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  await selected.click();
  await expect(page.getByRole('heading', { name: 'NAND flash', exact: true })).toBeVisible();
  expect(requests.length).toBe(before);
});
