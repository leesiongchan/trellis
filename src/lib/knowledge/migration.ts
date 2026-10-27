import type { ExplorationRepository } from './types';
import type { LibraryClient } from './library-client';
import type { LibraryCache } from './cache';
import type { BeginImport, Command, ImportView } from './library-types';
import { encodeLegacyImport } from './legacy-import';
import { newId } from './normalize';
export type MigrationSummary = {
  imported: number;
  skipped: number;
  failed: number;
  messages: string[];
  mapping: Record<string, { explorationId: string; nodes: Record<string, string> }>;
};
export async function migrateLegacy(
  repository: ExplorationRepository,
  client: LibraryClient,
  cache: LibraryCache,
  options: {
    explicitRestore?: boolean;
    signal?: AbortSignal;
    onProgress?: (message: string) => void;
  } = {}
): Promise<MigrationSummary> {
  const info = client.library;
  if (!info) throw new Error('Connect to the private library before importing browser copies.');
  const summary: MigrationSummary = {
    imported: 0,
    skipped: 0,
    failed: 0,
    messages: [],
    mapping: {}
  };
  for (const original of await repository.list()) {
    if (options.signal?.aborted) break;
    if (original.mode === 'demo') {
      summary.skipped++;
      continue;
    }
    try {
      options.onProgress?.('Importing saved browser exploration: ' + original.originalPrompt);
      const encoded = await encodeLegacyImport(original);
      const prior = await cache.receipt(info, original.id, encoded.digest);
      if (prior && !(options.explicitRestore && prior.state === 'suppressed')) {
        summary.skipped++;
        if (prior.explorationId)
          summary.mapping[original.id] = { explorationId: prior.explorationId, nodes: prior.nodes };
        continue;
      }
      let upload = await cache.upload(info, original.id, encoded.digest);
      if (!upload) {
        const command: Command<BeginImport> = {
          operationId: newId(),
          libraryEpoch: info.libraryEpoch,
          expectedRevision: null,
          payload: {
            legacyExplorationId: original.id,
            contentDigest: encoded.digest,
            expectedBytes: encoded.expectedBytes,
            expectedNodes: encoded.expectedNodes,
            explicitRestore: options.explicitRestore ?? false
          }
        };
        upload = { command, batchIds: encoded.batches.map(() => newId()), finishId: newId() };
        await cache.putUpload(info, original.id, encoded.digest, upload);
      }
      let view: ImportView = await client.beginImport(upload.command as Command<BeginImport>);
      if (view.state === 'staging' && view.uploadOperationId !== upload.command.operationId) {
        const deadline = Date.now() + 30_000;
        while (view.state === 'staging' && Date.now() < deadline && !options.signal?.aborted) {
          await new Promise((resolve) => setTimeout(resolve, 500));
          view = await client.importStatus(view.id, options.signal);
        }
        if (view.state === 'staging')
          throw new Error(
            'Another browser is importing this copy. Reconnect after it finishes; your original is preserved.'
          );
      } else if (view.state === 'staging') {
        for (const [batchIndex, records] of encoded.batches.entries()) {
          if (options.signal?.aborted)
            throw new Error('Import paused. The original browser copy is preserved.');
          view = await client.importBatch({
            operationId: upload.batchIds[batchIndex],
            libraryEpoch: info.libraryEpoch,
            expectedRevision: null,
            payload: { importId: view.id, batchIndex, records }
          });
        }
        view = await client.finishImport({
          operationId: upload.finishId,
          libraryEpoch: info.libraryEpoch,
          expectedRevision: null,
          payload: { importId: view.id }
        });
      }
      if (view.state === 'failed')
        throw new Error(
          view.error?.message ?? 'Import could not finish. Keep the original browser copy.'
        );
      if (view.state !== 'completed' && view.state !== 'suppressed')
        throw new Error('Import has not completed. Keep the original browser copy.');
      if (view.state === 'suppressed') {
        await cache.putReceipt(info, original.id, {
          digest: encoded.digest,
          explorationId: view.explorationId,
          nodes: view.mapping,
          state: view.state
        });
        await cache.removeUpload(info, original.id, encoded.digest);

        summary.skipped++;
        summary.messages.push(
          'A previously deleted or older-library copy was preserved on this device. Use Restore browser copies explicitly to import it again.'
        );
        continue;
      }
      if (!view.explorationId) throw new Error('The imported exploration mapping is missing.');
      const snapshot = await client.exploration(view.explorationId, options.signal);
      await cache.putSnapshot(info, snapshot);
      const selectedNodeId =
        view.mapping[original.view.selectedNodeId] ?? snapshot.exploration.rootId;
      await cache.putView(info, view.explorationId, {
        ...original.view,
        selectedNodeId,
        expandedNodeIds: original.view.expandedNodeIds.flatMap((id) =>
          view.mapping[id] ? [view.mapping[id]] : []
        )
      });
      await cache.putReceipt(info, original.id, {
        digest: encoded.digest,
        explorationId: view.explorationId,
        nodes: view.mapping,
        state: view.state
      });
      await cache.removeUpload(info, original.id, encoded.digest);
      summary.mapping[original.id] = { explorationId: view.explorationId, nodes: view.mapping };
      summary.imported++;
    } catch (error) {
      summary.failed++;
      summary.messages.push(
        error instanceof Error
          ? error.message
          : 'A browser copy could not be imported. Its original is preserved.'
      );
    }
  }
  return summary;
}
