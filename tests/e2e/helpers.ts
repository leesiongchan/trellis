import { expect, type Page } from '@playwright/test';
export const fixtureKey = 'fixture-access-key-'.repeat(4);
export async function authorize(page: Page) {
  await page.goto('/access?key=' + encodeURIComponent(fixtureKey));
}
// Observe genuine Rust requests and saved snapshots; all answers pass through embedded storage.
export async function observeResearch(page: Page, authenticate = true) {
  if (authenticate) await authorize(page);
  const questions = new Map<string, string>(),
    requests: string[] = [];
  page.on('response', async (response) => {
    if (/\/api\/explorations\/[^/]+$/.test(new URL(response.url()).pathname) && response.ok()) {
      try {
        const snapshot = await response.json();
        for (const a of snapshot.appearances ?? []) {
          questions.set(a.id, a.question);
          for (let i = 0; i < requests.length; i++)
            if (requests[i] === a.id) requests[i] = a.question;
        }
      } catch {
        /* disconnected response */
      }
    }
  });
  page.on('request', (request) => {
    if (/\/api\/appearances\/[^/]+\/research$/.test(new URL(request.url()).pathname)) {
      const input = request.postDataJSON();
      requests.push(questions.get(input.payload.appearanceId) ?? input.payload.appearanceId);
    }
  });
  return requests;
}
export async function startExample(page: Page) {
  await page.goto('/');
  await page.getByLabel('What would you like to understand?').fill('microSD cards');
  await page.getByRole('button', { name: 'Start exploring', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'microSD cards', exact: true, includeHidden: true })
  ).toBeAttached();
  await expect(
    page.getByRole('button', { name: 'Refresh research', exact: true, includeHidden: true })
  ).toBeAttached();
}

export async function seedLegacy(page: Page, values: unknown[]) {
  await page.evaluate(async (items) => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('branch-knowledge-v1', 1);
      request.onupgradeneeded = () =>
        request.result.createObjectStore('explorations', { keyPath: 'id' });
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    const tx = db.transaction('explorations', 'readwrite');
    for (const item of items) tx.objectStore('explorations').put(item);
    await new Promise<void>((resolve, reject) => {
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
    });
    db.close();
  }, values);
}
