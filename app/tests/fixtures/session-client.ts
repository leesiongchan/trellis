import { createLibraryClient } from '../../src/lib/knowledge/library-client';
import { jsonResponse } from './library';
export function createSessionClient(fetcher: typeof fetch) {
  return createLibraryClient(async (path, init) => {
    if (String(path).startsWith('/api/operations?'))
      return jsonResponse({ items: [], nextCursor: null });
    if (String(path).endsWith('/view'))
      return jsonResponse(
        init?.method === 'PUT' ? JSON.parse(init.body as string).payload.view : null
      );
    return fetcher(path, init);
  });
}
