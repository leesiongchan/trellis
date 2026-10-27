import { openDB, type DBSchema } from 'idb';
import { z } from 'zod';
import type { ExplorationSnapshot, LibraryInfo } from './library-types';
import type { ViewState } from './types';
import type { Intent } from './library-client';
import { ExplorationSnapshotSchema, LibraryInfoSchema, OpaqueIdSchema } from './library-schemas';
import { ViewSchema } from './schemas';
export type MigrationReceipt = {
  digest: string;
  explorationId: string | null;
  nodes: Record<string, string>;
  state: 'completed' | 'suppressed';
};
export type MigrationUpload = { command: Intent['command']; batchIds: string[]; finishId: string };
interface CacheDB extends DBSchema {
  records: { key: string; value: unknown };
}
const same = (a: LibraryInfo, b: LibraryInfo) =>
  a.protocolVersion === b.protocolVersion &&
  a.libraryId === b.libraryId &&
  a.libraryEpoch === b.libraryEpoch;
const prefix = (info: LibraryInfo) => JSON.stringify([2, info.libraryId, info.libraryEpoch]) + ':';
export function createLibraryCache(name = 'branch-library-v2') {
  const db = () =>
    openDB<CacheDB>(name, 1, {
      upgrade(database) {
        database.createObjectStore('records');
      }
    });
  async function get(key: string) {
    const database = await db();
    try {
      return await database.get('records', key);
    } finally {
      database.close();
    }
  }
  async function put(key: string, value: unknown) {
    const database = await db();
    try {
      await database.put('records', value, key);
    } finally {
      database.close();
    }
  }
  async function remove(key: string) {
    const database = await db();
    try {
      await database.delete('records', key);
    } finally {
      database.close();
    }
  }
  async function entries(info: LibraryInfo, kind: string) {
    const database = await db();
    try {
      const start = prefix(info) + kind + ':';
      return await database.getAll('records', IDBKeyRange.bound(start, start + '\uffff'));
    } finally {
      database.close();
    }
  }
  return {
    async lastLibrary() {
      const value = LibraryInfoSchema.safeParse(await get('library'));
      return value.success ? value.data : null;
    },
    async acceptLibrary(info: LibraryInfo) {
      LibraryInfoSchema.parse(info);
      const database = await db();
      try {
        const tx = database.transaction('records', 'readwrite');
        const old = LibraryInfoSchema.safeParse(await tx.store.get('library'));
        if (old.success && !same(old.data, info)) await tx.store.clear();
        await tx.store.put(info, 'library');
        await tx.done;
      } finally {
        database.close();
      }
    },
    async putSnapshot(info: LibraryInfo, snapshot: ExplorationSnapshot) {
      const value = ExplorationSnapshotSchema.parse(snapshot);
      if (value.libraryEpoch !== info.libraryEpoch) throw new Error('The library epoch changed.');
      await put(prefix(info) + 'snapshot:' + value.exploration.id, value);
    },
    async getSnapshot(info: LibraryInfo, id: string) {
      const result = ExplorationSnapshotSchema.safeParse(
        await get(prefix(info) + 'snapshot:' + id)
      );
      return result.success && result.data.libraryEpoch === info.libraryEpoch ? result.data : null;
    },
    async snapshots(info: LibraryInfo) {
      return (await entries(info, 'snapshot')).flatMap((value) => {
        const parsed = ExplorationSnapshotSchema.safeParse(value);
        return parsed.success && parsed.data.libraryEpoch === info.libraryEpoch
          ? [parsed.data]
          : [];
      });
    },
    putView: (info: LibraryInfo, id: string, view: ViewState) =>
      put(prefix(info) + 'view:' + id, ViewSchema.parse(view)),
    async getView(info: LibraryInfo, id: string) {
      const parsed = ViewSchema.safeParse(await get(prefix(info) + 'view:' + id));
      return parsed.success ? parsed.data : null;
    },
    async putIntent(intent: Intent) {
      OpaqueIdSchema.parse(intent.command.operationId);
      await put(prefix(intent.library) + 'intent:' + intent.command.operationId, intent);
    },
    async intents(info: LibraryInfo): Promise<Intent[]> {
      return (await entries(info, 'intent')).map((value) =>
        z
          .object({
            library: LibraryInfoSchema,
            path: z.string().startsWith('/api/'),
            method: z.enum(['POST', 'PATCH', 'DELETE']),
            command: z.object({
              operationId: OpaqueIdSchema,
              libraryEpoch: z.number().int().min(1),
              expectedRevision: z.number().int().nullable(),
              payload: z.unknown()
            })
          })
          .parse(value)
      );
    },
    removeIntent: (info: LibraryInfo, id: string) => remove(prefix(info) + 'intent:' + id),
    removeSnapshot: async (info: LibraryInfo, id: string) => {
      await remove(prefix(info) + 'snapshot:' + id);
      await remove(prefix(info) + 'view:' + id);
    },
    async receipt(
      info: LibraryInfo,
      legacyId: string,
      digest: string
    ): Promise<MigrationReceipt | null> {
      return (
        ((await get(
          prefix(info) + 'receipt:' + JSON.stringify([legacyId, digest])
        )) as MigrationReceipt) ?? null
      );
    },
    putReceipt: (info: LibraryInfo, legacyId: string, receipt: MigrationReceipt) =>
      put(prefix(info) + 'receipt:' + JSON.stringify([legacyId, receipt.digest]), receipt),
    async upload(
      info: LibraryInfo,
      legacyId: string,
      digest: string
    ): Promise<MigrationUpload | null> {
      return (
        ((await get(
          prefix(info) + 'upload:' + JSON.stringify([legacyId, digest])
        )) as MigrationUpload) ?? null
      );
    },
    putUpload: (info: LibraryInfo, legacyId: string, digest: string, upload: MigrationUpload) =>
      put(prefix(info) + 'upload:' + JSON.stringify([legacyId, digest]), upload),
    removeUpload: (info: LibraryInfo, legacyId: string, digest: string) =>
      remove(prefix(info) + 'upload:' + JSON.stringify([legacyId, digest])),
    async clear() {
      const database = await db();
      try {
        await database.clear('records');
      } finally {
        database.close();
      }
    }
  };
}
export type LibraryCache = ReturnType<typeof createLibraryCache>;
