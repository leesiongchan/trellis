import { expect, test } from './fixtures';
import { observeResearch, seedServerExploration, startExample } from './helpers';
import { makeContent, makeExploration } from '../fixtures/knowledge';

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

test('long cited explanations reflow and keep keyboard focus visible', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await observeResearch(page);
  const exploration = makeExploration({
    id: crypto.randomUUID(),
    originalPrompt: 'Readable long-form exploration'
  });
  const summary =
    'Flash retains data without power. ' +
    'A detailed explanation keeps the physical mechanism and its limitations connected. '.repeat(
      12
    );
  const sourceTitle = 'A detailed reference about the physical limits of flash memory storage';
  const sourceUrl = 'https://example.org/reference/' + 'a-long-reference-path-'.repeat(35);
  exploration.nodes.root.content = makeContent({
    title:
      'Understanding how a tiny physical device stores information, and how its design choices shape both capacity and reliability',
    summary: [
      {
        id: 'p1',
        text: summary,
        evidence: [{ sourceId: 's1', quote: 'Flash retains data without power.' }]
      }
    ],
    sources: [
      {
        id: 's1',
        title: sourceTitle,
        url: sourceUrl,
        publisher: 'A specialist reference publisher with a deliberately long name',
        retrievedAt: '2026-10-25T00:00:00.000Z'
      }
    ]
  });
  exploration.view.mobilePane = 'reader';
  await seedServerExploration(page, exploration);
  await page.reload();
  await page
    .getByRole('button', { name: 'Resume Readable long-form exploration', exact: true })
    .click();
  const reader = page.getByRole('complementary', { name: 'Topic explanation', exact: true });
  await expect(reader).toBeVisible();
  const paragraph = reader.getByText(summary, { exact: false }).first();
  expect(
    await paragraph.evaluate((element) => parseFloat(getComputedStyle(element).fontSize))
  ).toBeGreaterThanOrEqual(16);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  const citation = reader
    .getByRole('link', { name: 'Source 1: ' + sourceTitle, exact: true })
    .first();
  await reader.getByRole('button', { name: 'Refresh research', exact: true }).focus();
  await page.keyboard.press('Tab');
  await expect(citation).toBeFocused();
  await expect(citation).toHaveAttribute('href', sourceUrl);
  await expect(citation).toHaveAttribute('rel', 'noopener noreferrer');
  expect(
    await citation.evaluate((element) => {
      const style = getComputedStyle(element),
        rect = element.getBoundingClientRect();
      return (
        style.outlineStyle !== 'none' &&
        parseFloat(style.outlineWidth) >= 2 &&
        rect.top >= 0 &&
        rect.bottom <= innerHeight
      );
    })
  ).toBe(true);
  await page.keyboard.press('Tab');
  await expect(citation).not.toBeFocused();
  const question = reader.getByLabel('Your branch question');
  await question.focus();
  await question.fill(
    'A further question that should remain readable and editable on a narrow screen'
  );
  await expect(question).toBeFocused();
  const submit = reader.getByRole('button', { name: 'Add branch', exact: true });
  expect((await submit.boundingBox())!.height).toBeGreaterThanOrEqual(44);
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

test('200 percent text reflows on a 320px screen', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await observeResearch(page);
  await page.evaluate(() => (document.documentElement.style.fontSize = '200%'));
  await expect(
    page.getByRole('heading', { name: 'A little curiosity.', exact: false })
  ).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.getByLabel('What would you like to understand?').fill('microSD cards');
  await page.getByRole('button', { name: 'Start exploring', exact: true }).click();
  await page.getByRole('button', { name: 'Read', exact: true }).click();
  const reader = page.getByRole('complementary', { name: 'Topic explanation', exact: true });
  await expect(reader.getByRole('heading', { name: 'microSD cards', exact: true })).toBeVisible();
  const input = reader.getByLabel('Your branch question');
  await input.scrollIntoViewIfNeeded();
  await input.fill('How does the controller work?');
  await expect(input).toBeEditable();
  const submit = reader.getByRole('button', { name: 'Add branch', exact: true });
  await submit.scrollIntoViewIfNeeded();
  expect((await submit.boundingBox())!.width).toBeGreaterThanOrEqual(44);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('saved explorations stay inside the tablet viewport with enlarged text', async ({ page }) => {
  await page.setViewportSize({ width: 768, height: 1024 });
  await observeResearch(page);
  await startExample(page);
  await page.evaluate(() => (document.documentElement.style.fontSize = '200%'));
  await page.getByRole('button', { name: 'Your explorations', exact: true }).click();
  const drawer = page.getByRole('region', { name: 'Your explorations drawer', exact: true });
  await expect(drawer).toBeVisible();
  const bounds = (await drawer.boundingBox())!;
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(768);
  await expect(
    drawer.getByRole('button', { name: 'Resume microSD cards', exact: true })
  ).toBeVisible();
});

test('failed research is identified in the map without relying on color', async ({ page }) => {
  await observeResearch(page);
  await startExample(page);
  const question = 'A new research question ' + crypto.randomUUID();
  await page.route('**/api/appearances/*/research', (route) => route.abort());
  await page.getByLabel('Your branch question').fill(question);
  await page.getByRole('button', { name: 'Add branch', exact: true }).click();
  const read = page.getByRole('button', { name: 'Read ' + question, exact: true });
  const status = 'Research failed. Open to retry.';
  await expect(read.getByText(status, { exact: true })).toBeVisible();
  await expect(read).toHaveAccessibleDescription(status);
  await expect(page.getByRole('button', { name: 'Expand ' + question, exact: true })).toBeVisible();
});
