import { expect, test } from './fixtures';
import { observeResearch, seedServerExploration } from './helpers';
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
