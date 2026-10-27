import { openDB, type DBSchema } from 'idb';
import { ExplorationSchema } from './schemas';
import type { Exploration, ExplorationRepository } from './types';

interface KnowledgeDB extends DBSchema {
  explorations: { key: string; value: Exploration };
}
export class SaveConflict extends Error {
  constructor() {
    super('This exploration changed in another tab. Reload the saved version before continuing.');
  }
}
export function restoreExploration(value: unknown): Exploration {
  const result = ExplorationSchema.parse(value);
  for (const node of Object.values(result.nodes)) {
    if (node.status === 'researching' || node.isRefreshing) {
      node.status = node.content ? 'ready' : 'failed';
      node.isRefreshing = false;
      node.lastError = {
        code: 'INTERRUPTED',
        message: 'Research was interrupted. Your earlier content is safe; retry when ready.',
        retryable: true,
        requestId: 'local'
      };
    }
  }
  return result;
}
export async function readBrowserOriginals(name = 'branch-knowledge-v1'): Promise<unknown[]> {
  const database = await openDB<KnowledgeDB>(name, 1, {
    upgrade(db) {
      db.createObjectStore('explorations', { keyPath: 'id' });
    }
  });
  try {
    return await database.getAll('explorations');
  } finally {
    database.close();
  }
}
export function createExplorationRepository(
  name = 'branch-knowledge-v1',
  onUnreadable: (message: string) => void = () => {}
): ExplorationRepository {
  const db = () =>
    openDB<KnowledgeDB>(name, 1, {
      upgrade(database) {
        database.createObjectStore('explorations', { keyPath: 'id' });
      }
    });
  return {
    async list() {
      const database = await db();
      try {
        const healthy: Exploration[] = [];
        let unreadable = false;
        for (const value of await database.getAll('explorations')) {
          try {
            healthy.push(restoreExploration(value));
          } catch {
            unreadable = true;
          }
        }
        if (unreadable) {
          onUnreadable(
            'Some saved explorations could not be read. Your browser data has been preserved.'
          );
        }
        return healthy.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
      } finally {
        database.close();
      }
    },
    async get(id) {
      const database = await db();
      try {
        const value = await database.get('explorations', id);
        return value ? restoreExploration(value) : null;
      } finally {
        database.close();
      }
    },
    async save(value, expectedRevision) {
      const parsed = ExplorationSchema.parse(value),
        database = await db();
      try {
        const tx = database.transaction('explorations', 'readwrite'),
          existing = await tx.store.get(value.id);
        if ((existing?.revision ?? 0) !== expectedRevision) {
          tx.abort();
          await tx.done.catch(() => {});
          throw new SaveConflict();
        }
        const next = { ...parsed, revision: expectedRevision + 1 };
        await tx.store.put(next);
        await tx.done;
        return next;
      } finally {
        database.close();
      }
    },
    async remove(id) {
      const database = await db();
      try {
        await database.delete('explorations', id);
      } finally {
        database.close();
      }
    }
  };
}
export type SaveStatus = 'saving' | 'saved' | 'failed' | 'conflict';
export function createAutosave(
  repository: ExplorationRepository,
  onStatus: (status: SaveStatus, message?: string) => void
) {
  let pending: Exploration | null = null,
    timer: ReturnType<typeof setTimeout> | undefined,
    active: Promise<void> | null = null,
    disposed = false,
    hasFailed = false;
  const revisions = new Map<string, number>();
  const run = async () => {
    while (pending && !disposed) {
      const value = pending;
      pending = null;
      onStatus('saving');
      try {
        const saved = await repository.save(value, revisions.get(value.id) ?? value.revision);
        revisions.set(value.id, saved.revision);
        hasFailed = false;
        if (!disposed && !pending) onStatus('saved');
      } catch (error) {
        if (!pending) pending = value;
        hasFailed = true;
        clearTimeout(timer);
        const message =
          error instanceof Error ? error.message : 'Your browser could not save this exploration.';
        if (!disposed) onStatus(error instanceof SaveConflict ? 'conflict' : 'failed', message);
        throw error;
      }
    }
  };
  const flush = async () => {
    clearTimeout(timer);
    if (disposed) return;
    if (active) {
      await active;
      if (pending) return flush();
      return;
    }
    active = run();
    try {
      await active;
    } finally {
      active = null;
    }
  };
  return {
    enqueue(value: Exploration) {
      if (disposed) return;
      pending = structuredClone(value);
      clearTimeout(timer);
      if (hasFailed) return;
      onStatus('saving');
      timer = setTimeout(() => {
        void flush().catch(() => {});
      }, 300);
    },
    flush,
    dispose() {
      disposed = true;
      pending = null;
      clearTimeout(timer);
    }
  };
}
