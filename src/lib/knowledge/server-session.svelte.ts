import { LibraryError, type Intent, type LibraryClient } from './library-client';
import type { LibraryCache } from './cache';
import type { Command, ExplorationSnapshot, LibraryInfo, OperationView } from './library-types';
import type { ApiError, ViewState, WorkspaceExploration, WorkspaceSession } from './types';
import { newId, normalizeQuestion } from './normalize';
import { ancestorPath } from './tree';
import {
  CreateExplorationCommandSchema,
  AddBranchCommandSchema,
  ResearchCommandSchema
} from './library-schemas';
const questionKey = (value: string) =>
  value.normalize('NFC').trim().toLowerCase().replace(/\s+/gu, ' ');
const errorView = (error: unknown): ApiError => ({
  code: error instanceof LibraryError ? error.code : 'LIBRARY_FAILED',
  message: error instanceof Error ? error.message : 'The library request did not finish.',
  retryable: error instanceof LibraryError ? error.retryable : true,
  requestId: error instanceof LibraryError ? error.requestId : 'local'
});
export function projectSnapshot(
  snapshot: ExplorationSnapshot,
  view?: ViewState | null
): WorkspaceExploration {
  const answers = new Map(snapshot.answers.map((answer) => [answer.id, answer]));
  const nodes = Object.fromEntries(
    snapshot.appearances.map((a) => {
      const content = a.answerVersionId ? (answers.get(a.answerVersionId)?.content ?? null) : null;
      return [
        a.id,
        {
          id: a.id,
          parentId: a.parentId,
          question: a.question,
          relationship: a.relationship,
          teaser: a.teaser,
          content,
          childIds: a.childIds,
          topicId: a.topicId,
          answerVersionId: a.answerVersionId,
          status: content ? ('ready' as const) : ('suggested' as const),
          lastError: null,
          isRefreshing: false
        }
      ];
    })
  );
  const root = snapshot.exploration.rootId;
  return {
    ...snapshot.exploration,
    mode: 'research',
    viewRestored: !!view,
    libraryEpoch: snapshot.libraryEpoch,
    nodes,
    view: {
      selectedNodeId: view && nodes[view.selectedNodeId] ? view.selectedNodeId : root,
      expandedNodeIds: view ? view.expandedNodeIds.filter((id) => nodes[id]) : [root],
      viewport: view?.viewport ?? { x: 0, y: 0, zoom: 1 },
      mobilePane: view?.mobilePane ?? 'map'
    }
  };
}
export class ResearchSession implements WorkspaceSession {
  exploration = $state.raw<WorkspaceExploration | null>(null);
  clarification = $state.raw<{ nodeId: string; question: string } | null>(null);
  notice = $state('');
  offline = $state(false);
  busy = $state(false);
  viewSaving = $state(false);
  viewError = $state('');
  private viewTail: Promise<void> = Promise.resolve();
  private viewSerial = 0;
  private recoveringCount = $state(0);
  get recovering() {
    return this.recoveringCount > 0;
  }
  private openedPins = new Set<string>();
  conflict = $state(false);
  library = $state.raw<LibraryInfo | null>(null);
  private generation = 0;
  private controller = new AbortController();
  private tail: Promise<unknown> = Promise.resolve();
  constructor(
    private options: {
      client: LibraryClient;
      cache: LibraryCache;
      onChange: (value: WorkspaceExploration) => void;
    }
  ) {}
  private change(value: WorkspaceExploration) {
    this.exploration = value;
    this.options.onChange(value);
  }
  private patchNode(id: string, change: (node: WorkspaceExploration['nodes'][string]) => void) {
    if (!this.exploration?.nodes[id]) return;
    const value = structuredClone(this.exploration);
    change(value.nodes[id]);
    this.change(value);
  }
  get readOnly() {
    return this.offline || this.conflict;
  }
  private requireOnline() {
    if (this.offline || !this.library)
      throw new LibraryError(
        'OFFLINE',
        'This saved copy is offline and read-only. Reconnect before making changes.'
      );
    if (this.conflict)
      throw new LibraryError(
        'REVISION_CONFLICT',
        'Reload the saved version before making more changes.'
      );
  }
  async connect(recover = true) {
    try {
      const info = await this.options.client.bootstrap(this.controller.signal);
      await this.options.cache.acceptLibrary(info);
      const changed =
        this.library &&
        (this.library.libraryId !== info.libraryId ||
          this.library.libraryEpoch !== info.libraryEpoch);
      this.library = info;
      this.offline = false;
      if (changed) {
        this.generation++;
        this.exploration = null;
        this.clarification = null;
        this.notice =
          'The library changed. Older cached copies and pending commands were cleared on this device.';
      }
      if (recover) await this.recover();
    } catch (error) {
      if (error instanceof LibraryError && error.code === 'OFFLINE') {
        this.offline = true;
        this.library = await this.options.cache.lastLibrary();
        this.notice = error.message;
        return;
      }
      throw error;
    }
  }
  async load(id: string) {
    const generation = ++this.generation;
    await this.flushView();
    this.clarification = null;
    const info = this.library;
    if (!info) throw new LibraryError('OFFLINE', 'Connect to your library to open an exploration.');
    let snapshot: ExplorationSnapshot | null;
    if (this.offline) snapshot = await this.options.cache.getSnapshot(info, id);
    else {
      try {
        snapshot = await this.options.client.exploration(id, this.controller.signal);
        await this.options.cache.putSnapshot(info, snapshot);
      } catch (error) {
        if (!(error instanceof LibraryError) || error.code !== 'OFFLINE') throw error;
        this.offline = true;
        this.notice = error.message;
        snapshot = await this.options.cache.getSnapshot(info, id);
      }
    }
    if (!snapshot)
      throw new Error('This exploration has no saved copy on this device. Reconnect to open it.');
    const view =
      this.exploration?.id === id
        ? this.exploration.view
        : await this.options.cache.getView(info, id);
    if (generation === this.generation) {
      this.conflict = false;
      this.change(projectSnapshot(snapshot, view));
    }
  }
  private async apply(operation: OperationView, generation: number) {
    if (operation.state !== 'completed')
      throw new LibraryError(
        operation.error?.code ?? 'WORK_INTERRUPTED',
        operation.error?.message ?? 'Research was interrupted. Retry explicitly when ready.',
        operation.error?.retryable ?? true
      );
    const result = operation.result;
    if (generation !== this.generation || !result) return;
    if ('explorationId' in result && result.kind !== 'deletion') {
      const info = this.library!;
      let snapshot: ExplorationSnapshot;
      try {
        snapshot = await this.options.client.exploration(
          result.explorationId,
          this.controller.signal
        );
      } catch (error) {
        if (!(error instanceof LibraryError) || error.status !== 404) throw error;
        await this.options.cache.removeSnapshot(info, result.explorationId);
        if (generation === this.generation) {
          this.notice =
            'This exploration was deleted in another tab. Start a new exploration to continue.';
          if (this.exploration?.id === result.explorationId) {
            this.generation++;
            this.exploration = null;
            this.conflict = false;
            this.clarification = null;
          }
        }
        return;
      }
      await this.options.cache.putSnapshot(info, snapshot);
      const view =
        this.exploration?.id === snapshot.exploration.id
          ? this.exploration.view
          : await this.options.cache.getView(info, snapshot.exploration.id);
      if (generation !== this.generation) return;
      this.change(projectSnapshot(snapshot, view));
      if (result.kind === 'research' && result.outcome.kind === 'clarification')
        this.clarification = { nodeId: result.appearanceId, question: result.outcome.question };
      if (result.kind === 'research' && result.outcome.kind === 'insufficient-evidence')
        this.patchNode(result.appearanceId, (node) => {
          node.status = node.content ? 'ready' : 'failed';
          node.lastError = {
            code: 'INSUFFICIENT_EVIDENCE',
            message: result.outcome.kind === 'insufficient-evidence' ? result.outcome.message : '',
            retryable: true,
            requestId: operation.id
          };
        });
    }
  }
  private async finish(intent: Intent, known?: OperationView, signal = this.controller.signal) {
    signal.throwIfAborted();
    let operation = known ?? (await this.options.client.submit(intent, signal));
    let delay = 250;
    while (operation.state === 'pending') {
      await new Promise<void>((resolve, reject) => {
        if (signal.aborted) {
          reject(signal.reason);
          return;
        }
        const stop = () => {
          clearTimeout(timer);
          reject(signal.reason);
        };
        const timer = setTimeout(() => {
          signal.removeEventListener('abort', stop);
          resolve();
        }, delay);
        signal.addEventListener('abort', stop, { once: true });
      });
      operation = await this.options.client.operation(operation.id, signal);
      delay = Math.min(2000, delay * 2);
    }
    if (operation.state !== 'completed')
      await this.options.cache.removeIntent(intent.library, intent.command.operationId);
    return operation;
  }
  async recover() {
    this.recoveringCount++;
    try {
      if (this.offline) await this.connect(false);
      this.requireOnline();
      const generation = this.generation;
      const signal = this.controller.signal;
      for (const intent of await this.options.cache.intents(this.library!)) {
        signal.throwIfAborted();
        if (generation !== this.generation) return;
        let operation: OperationView | undefined;
        try {
          operation = await this.options.client.operation(intent.command.operationId, signal);
        } catch (error) {
          if (!(error instanceof LibraryError) || error.status !== 404) throw error;
        }
        operation = await this.finish(intent, operation, signal);
        if (operation.state !== 'completed') {
          this.notice =
            operation.error?.message ?? 'Research was interrupted. Retry explicitly when ready.';
          continue;
        }
        await this.apply(operation, generation);
        await this.options.cache.removeIntent(intent.library, intent.command.operationId);
      }
    } finally {
      this.recoveringCount--;
    }
  }

  private async execute(
    path: string,
    method: string,
    command: Command<unknown>,
    generation: number
  ) {
    this.requireOnline();
    if (generation !== this.generation) throw new Error('The open exploration changed.');
    const signal = this.controller.signal;
    const intent = { library: this.library!, path, method, command };
    await this.options.cache.putIntent(intent);
    try {
      signal.throwIfAborted();
      if (generation !== this.generation) throw new Error('The open exploration changed.');
      const operation = await this.finish(intent, undefined, signal);
      await this.apply(operation, generation);
      await this.options.cache.removeIntent(intent.library, intent.command.operationId);
      return operation;
    } catch (error) {
      if (
        error instanceof LibraryError &&
        error.status >= 400 &&
        error.status < 500 &&
        error.status !== 401 &&
        error.status !== 429
      )
        await this.options.cache.removeIntent(intent.library, intent.command.operationId);
      if (
        error instanceof LibraryError &&
        error.code === 'REVISION_CONFLICT' &&
        generation === this.generation &&
        this.exploration &&
        (path === '/api/explorations/' + this.exploration.id ||
          path.startsWith('/api/appearances/'))
      )
        this.conflict = true;
      if (
        generation === this.generation &&
        error instanceof LibraryError &&
        error.code === 'OFFLINE'
      )
        this.offline = true;
      throw error;
    }
  }
  private action<T>(work: () => Promise<T>): Promise<T> {
    const generation = this.generation;
    const run = this.tail
      .catch(() => {})
      .then(async () => {
        if (generation !== this.generation) throw new Error('The open exploration changed.');
        this.busy = true;
        try {
          this.requireOnline();
          await this.recover();
          return await work();
        } finally {
          this.busy = false;
        }
      });
    this.tail = run;
    return run;
  }
  private command<T>(payload: T, revision: number | null): Command<T> {
    return {
      operationId: newId(),
      libraryEpoch: this.library!.libraryEpoch,
      expectedRevision: revision,
      payload
    };
  }
  start(question: string) {
    return this.action(async () => {
      this.requireOnline();
      const generation = ++this.generation;
      this.clarification = null;
      const command = CreateExplorationCommandSchema.parse(
        this.command({ question: normalizeQuestion(question), locale: 'en' }, null)
      );
      await this.execute('/api/explorations', 'POST', command, generation);
      if (generation === this.generation && this.exploration)
        await this.research(this.exploration.rootId, false, null, true);
    });
  }
  async select(id: string) {
    if (!this.exploration?.nodes[id]) return;
    this.setView({ selectedNodeId: id, mobilePane: 'reader' });
    if (!this.offline && (!this.exploration.nodes[id].content || this.needsSuggestions(id)))
      await this.action(() => this.research(id, false, null, true));
  }
  async expand(id: string) {
    if (!this.exploration?.nodes[id]) return;
    this.setView({
      selectedNodeId: id,
      expandedNodeIds: [...new Set([...this.exploration.view.expandedNodeIds, id])]
    });
    if (!this.offline && (!this.exploration.nodes[id].content || this.needsSuggestions(id)))
      await this.action(() => this.research(id, false, null, true));
  }
  collapse(id: string) {
    if (this.exploration)
      this.setView({
        expandedNodeIds: this.exploration.view.expandedNodeIds.filter((key) => key !== id)
      });
  }
  setView(patch: Partial<ViewState>) {
    if (!this.exploration || !this.library) return;
    const old = this.exploration.view;
    if (JSON.stringify({ ...old, ...patch }) === JSON.stringify(old)) return;
    this.change({ ...this.exploration, view: { ...old, ...patch } });
    this.persistView();
  }
  private persistView() {
    if (!this.library || !this.exploration) return;
    const info = this.library,
      id = this.exploration.id,
      view = structuredClone(this.exploration.view),
      serial = ++this.viewSerial;
    this.viewSaving = true;
    this.viewTail = this.viewTail
      .catch(() => {})
      .then(() => this.options.cache.putView(info, id, view));
    void this.viewTail.then(
      () => {
        if (serial === this.viewSerial) {
          this.viewSaving = false;
          this.viewError = '';
        }
      },
      (error) => {
        if (serial === this.viewSerial) {
          this.viewSaving = false;
          this.viewError =
            error instanceof Error
              ? error.message
              : 'The view position could not be saved on this device.';
          this.notice = this.viewError;
        }
      }
    );
  }
  flushView() {
    return this.viewTail;
  }
  async retryView() {
    this.persistView();
    await this.flushView();
  }

  addQuestion(parentId: string, question: string) {
    return this.action(async () => {
      this.requireOnline();
      const e = this.exploration;
      if (!e?.nodes[parentId]) throw new Error('Choose a parent topic first.');
      const normalized = normalizeQuestion(question),
        key = questionKey(normalized);
      const sibling = e.nodes[parentId].childIds.find(
        (id) => questionKey(e.nodes[id].question) === key
      );
      if (sibling) {
        this.notice = 'That question already has a branch. Opening it here.';
        this.setView({ selectedNodeId: sibling, mobilePane: 'reader' });
        return sibling;
      }
      if (ancestorPath(e, parentId).some((node) => questionKey(node.question) === key))
        throw new Error('That topic is already on this path. Select it in the breadcrumb.');
      const generation = this.generation;
      const command = AddBranchCommandSchema.parse(
        this.command({ parentAppearanceId: parentId, question: normalized }, e.revision)
      );
      const operation = await this.execute(
        `/api/explorations/${e.id}/appearances`,
        'POST',
        command,
        generation
      );
      if (operation.result?.kind !== 'exploration')
        throw new Error('The new branch was not returned.');
      const id = operation.result.appearanceId;
      this.setView({
        selectedNodeId: id,
        expandedNodeIds: [...new Set([...this.exploration!.view.expandedNodeIds, parentId])],
        mobilePane: 'reader'
      });
      await this.research(id, false);
      return id;
    });
  }
  refresh(id: string) {
    return this.action(() => this.research(id, true));
  }
  clarify(id: string, answer: string) {
    return this.action(() => this.research(id, true, normalizeQuestion(answer)));
  }
  private needsSuggestions(id: string) {
    const node = this.exploration?.nodes[id];
    return (
      !!node?.content?.branches.length &&
      !node.childIds.length &&
      !this.openedPins.has(id + ':' + node.answerVersionId)
    );
  }
  private async research(
    id: string,
    refresh: boolean,
    clarification: string | null = null,
    allowPinnedReuse = false
  ) {
    this.requireOnline();
    const e = this.exploration,
      node = e?.nodes[id];
    if (!e || !node || (node.content && !refresh && !allowPinnedReuse)) return;
    const generation = this.generation;
    this.clarification = null;
    const command = ResearchCommandSchema.parse(
      this.command({ appearanceId: id, refresh, clarification }, e.revision)
    );
    this.patchNode(id, (node) => {
      node.lastError = null;
      if (node.content) node.isRefreshing = true;
      else node.status = 'researching';
    });
    try {
      await this.execute(`/api/appearances/${id}/research`, 'POST', command, generation);
      this.openedPins.add(id + ':' + this.exploration?.nodes[id]?.answerVersionId);
    } catch (error) {
      if (generation === this.generation)
        this.patchNode(id, (node) => {
          node.isRefreshing = false;
          node.status = node.content ? 'ready' : 'failed';
          node.lastError = errorView(error);
        });
      throw error;
    }
  }
  mutate(path: string, method: string, payload: unknown, revision: number | null) {
    return this.action(() => {
      this.requireOnline();
      return this.execute(path, method, this.command(payload, revision), this.generation);
    });
  }
  dispose() {
    this.generation++;
    this.controller.abort();
    this.controller = new AbortController();
    this.clarification = null;
  }
}
export const createResearchSession = (options: ConstructorParameters<typeof ResearchSession>[0]) =>
  new ResearchSession(options);
