import { z } from 'zod';
import * as s from './library-schemas';
import type * as t from './library-types';
import { ApiErrorSchema, ViewSchema } from './schemas';
import type { ViewState } from './types';
import { newId } from './normalize';
export class LibraryError extends Error {
  constructor(
    public code: string,
    message: string,
    public retryable = false,
    public requestId = 'local',
    public status = 0
  ) {
    super(message);
  }
}
export type Intent = {
  library: t.LibraryInfo;
  path: string;
  method: string;
  command: t.Command<unknown>;
};
export function createLibraryClient(fetcher: typeof fetch = fetch) {
  let scope: t.LibraryInfo | null = null;
  async function request<T>(
    path: string,
    schema: z.ZodType<T>,
    init: RequestInit = {}
  ): Promise<T> {
    const headers = new Headers(init.headers);
    headers.set('X-Branch-Protocol', '2');
    headers.set('X-Request-Id', newId());
    if (scope) headers.set('X-Branch-Library', scope.libraryId);
    if (init.body) headers.set('Content-Type', 'application/json');
    let response: Response;
    try {
      response = await fetcher(path, {
        ...init,
        headers,
        cache: 'no-store',
        credentials: 'same-origin'
      });
    } catch (error) {
      if (init.signal?.aborted) throw error;
      throw new LibraryError(
        'OFFLINE',
        'Cannot reach the server. Reconnect to load or change saved content.',
        true
      );
    }
    if (response.headers.get('X-Branch-Protocol') !== '2')
      throw new LibraryError(
        'RELOAD_REQUIRED',
        'Reload this page to use the current library protocol.'
      );
    let value: unknown;
    try {
      value = await response.json();
    } catch {
      throw new LibraryError('INVALID_RESPONSE', 'The library sent an unreadable response.', true);
    }
    if (!response.ok) {
      const error = ApiErrorSchema.safeParse((value as { error?: unknown })?.error);
      if (error.success)
        throw new LibraryError(
          error.data.code,
          error.data.message,
          error.data.retryable,
          error.data.requestId,
          response.status
        );
      throw new LibraryError(
        'INVALID_RESPONSE',
        'The library request could not finish.',
        true,
        'local',
        response.status
      );
    }
    const parsed = schema.safeParse(value);
    if (!parsed.success)
      throw new LibraryError(
        'INVALID_RESPONSE',
        'The saved library response is incomplete. Reload before continuing.'
      );
    return parsed.data;
  }
  const read = <T>(path: string, schema: z.ZodType<T>, signal?: AbortSignal) =>
    request(path, schema, { signal });
  const write = <T>(path: string, method: string, command: t.Command<T>) =>
    request(path, s.OperationViewSchema, { method, body: JSON.stringify(command) });
  const id = (value: string) => encodeURIComponent(s.OpaqueIdSchema.parse(value));
  const query = (values: Record<string, string | number | undefined>) => {
    const params = new URLSearchParams();
    for (const [k, v] of Object.entries(values)) if (v !== undefined) params.set(k, String(v));
    return params.size ? '?' + params.toString() : '';
  };
  return {
    get library() {
      return scope;
    },
    async bootstrap(signal?: AbortSignal) {
      scope = null;
      const info = await read('/api/library', s.LibraryInfoSchema, signal);
      scope = info;
      return info;
    },
    setScope(info: t.LibraryInfo) {
      scope = s.LibraryInfoSchema.parse(info);
    },
    exploration: (key: string, signal?: AbortSignal) =>
      read(`/api/explorations/${id(key)}`, s.ExplorationSnapshotSchema, signal),
    explorations: (cursor?: string, limit = 50, signal?: AbortSignal) =>
      read(
        '/api/explorations' + query({ cursor, limit }),
        s.pageSchema(s.ExplorationSummarySchema),
        signal
      ),
    topics: (search?: string, cursor?: string, signal?: AbortSignal) =>
      read(
        '/api/topics' + query({ search, cursor, limit: 50 }),
        s.pageSchema(s.TopicSummarySchema),
        signal
      ),
    topic: (key: string, answerVersionId?: string, signal?: AbortSignal) =>
      read(`/api/topics/${id(key)}` + query({ answerVersionId }), s.TopicViewSchema, signal),
    connections: (key: string, cursor?: string, signal?: AbortSignal) =>
      read(
        `/api/topics/${id(key)}/connections` + query({ cursor, limit: 5 }),
        s.pageSchema(s.ConnectionViewSchema),
        signal
      ),
    appearances: (key: string, cursor?: string, answerVersionId?: string, signal?: AbortSignal) =>
      read(
        `/api/topics/${id(key)}/appearances` + query({ cursor, answerVersionId, limit: 5 }),
        s.pageSchema(s.AppearanceLocationSchema),
        signal
      ),
    neighborhood: (key: string, signal?: AbortSignal) =>
      read(`/api/topics/${id(key)}/neighborhood`, s.GraphNeighborhoodSchema, signal),
    view: (key: string, signal?: AbortSignal) =>
      read(`/api/explorations/${id(key)}/view`, ViewSchema.nullable(), signal),
    saveView: (key: string, command: t.Command<{ explorationId: string; view: ViewState }>) =>
      request(`/api/explorations/${id(key)}/view`, ViewSchema, {
        method: 'PUT',
        body: JSON.stringify(command)
      }),
    operations: (cursor?: string, signal?: AbortSignal) =>
      read(
        '/api/operations' + query({ cursor, limit: 50 }),
        s.pageSchema(s.OperationViewSchema),
        signal
      ),
    operation: (key: string, signal?: AbortSignal) =>
      read(`/api/operations/${id(key)}`, s.OperationViewSchema, signal),
    submit: (intent: Intent, signal?: AbortSignal) => {
      if (
        !scope ||
        intent.library.libraryId !== scope.libraryId ||
        intent.library.libraryEpoch !== scope.libraryEpoch ||
        intent.command.libraryEpoch !== scope.libraryEpoch
      )
        throw new LibraryError(
          'LIBRARY_EPOCH_CHANGED',
          'The library changed. Reload before continuing.'
        );
      return request(intent.path, s.OperationViewSchema, {
        method: intent.method,
        body: JSON.stringify(intent.command),
        signal
      });
    },
    create: (command: t.Command<t.CreateExploration>) =>
      request('/api/explorations', s.OperationViewSchema, {
        method: 'POST',
        body: JSON.stringify(s.CreateExplorationCommandSchema.parse(command))
      }),
    branch: (explorationId: string, command: t.Command<t.AddBranch>) =>
      write(
        `/api/explorations/${id(explorationId)}/appearances`,
        'POST',
        s.AddBranchCommandSchema.parse(command)
      ),
    research: (command: t.Command<t.ResearchInput>) =>
      write(
        `/api/appearances/${id(command.payload.appearanceId)}/research`,
        'POST',
        s.ResearchCommandSchema.parse(command)
      ),
    choose: (command: t.Command<t.ChooseAnswer>) =>
      write(
        `/api/appearances/${id(command.payload.appearanceId)}/answer`,
        'PATCH',
        s.ChooseAnswerCommandSchema.parse(command)
      ),
    startFromTopic: (command: t.Command<t.StartFromTopic>) =>
      write(
        `/api/topics/${id(command.payload.topicId)}/explorations`,
        'POST',
        s.StartFromTopicCommandSchema.parse(command)
      ),
    dismiss: (command: t.Command<t.DismissConnection>) =>
      write(
        `/api/connections/${id(command.payload.connectionId)}`,
        'PATCH',
        s.DismissConnectionCommandSchema.parse(command)
      ),
    remove: (command: t.Command<t.DeleteExploration>) =>
      write(
        `/api/explorations/${id(command.payload.explorationId)}`,
        'DELETE',
        s.DeleteExplorationCommandSchema.parse(command)
      ),
    purge: (command: t.Command<t.PurgeLibrary>) =>
      write('/api/library/purge', 'POST', s.PurgeLibraryCommandSchema.parse(command))
  };
}
export type LibraryClient = ReturnType<typeof createLibraryClient>;
