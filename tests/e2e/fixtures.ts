import { test as base, expect } from '@playwright/test';
import { fixtureKey } from './helpers';
export { expect };
// Each browser scenario receives an empty exploration list in disposable native storage.
// Retained knowledge remains available for exact cache reuse, matching production deletion semantics.
export const test = base.extend<{ isolatedLibrary: void }>({
  isolatedLibrary: [
    async ({ context, baseURL }, use) => {
      const access = await context.request.get(
        baseURL + '/access?key=' + encodeURIComponent(fixtureKey)
      );
      expect(access.ok()).toBe(true);
      const info = await (
        await context.request.get(baseURL + '/api/library', {
          headers: { 'X-Branch-Protocol': '2' }
        })
      ).json();
      const headers = {
        'X-Branch-Protocol': '2',
        'X-Branch-Library': info.libraryId,
        origin: baseURL!
      };
      for (let pass = 0; pass < 20; pass++) {
        const page = await (
          await context.request.get(baseURL + '/api/explorations?limit=100', { headers })
        ).json();
        if (!page.items.length) break;
        for (const item of page.items) {
          const response = await context.request.delete(baseURL + '/api/explorations/' + item.id, {
            headers,
            data: {
              operationId: crypto.randomUUID(),
              libraryEpoch: info.libraryEpoch,
              expectedRevision: item.revision,
              payload: { explorationId: item.id }
            }
          });
          if (response.status() === 409) continue; // A previous admitted save can finish during cleanup.
          expect(response.ok(), await response.text()).toBe(true);
        }
      }
      await use();
    },
    { auto: true }
  ]
});
