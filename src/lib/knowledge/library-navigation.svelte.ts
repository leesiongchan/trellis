import type { LibraryClient } from './library-client';
import type { ResearchSession } from './server-session.svelte';
import type { TopicView, Page, ConnectionView, TopicSummary } from './library-types';
import type { ViewState } from './types';
type Location =
  | { kind: 'exploration'; id: string; view: ViewState }
  | { kind: 'topic'; id: string; version?: string };
export class LibraryNavigation {
  topic = $state.raw<TopicView | null>(null);
  retained = $state.raw<TopicView | null>(null);
  connections = $state.raw<Page<ConnectionView>>({ items: [], nextCursor: null });
  catalog = $state.raw<Page<TopicSummary>>({ items: [], nextCursor: null });
  history = $state.raw<Location[]>([]);
  showCatalog = $state(false);
  search = $state('');
  error = $state('');
  loading = $state(false);
  private transition = 0;
  private catalogGeneration = 0;
  private topicId: string | null = null;
  private version: string | undefined;
  private key = '';
  private generation = 0;
  private paginated = false;
  private fetching = false;
  private controller = new AbortController();
  private interval?: ReturnType<typeof setInterval>;
  private searchTimer?: ReturnType<typeof setTimeout>;
  constructor(
    private client: LibraryClient,
    private session: ResearchSession,
    private onNavigate: () => void
  ) {}
  start() {
    this.interval = setInterval(() => {
      if (this.topicId && !this.session.offline && !this.fetching) void this.refresh(false);
    }, 2000);
  }
  track(id: string | null, version?: string) {
    const key = JSON.stringify([
      id,
      version,
      this.session.offline,
      this.session.library?.libraryId,
      this.session.library?.libraryEpoch
    ]);
    if (key === this.key) return;
    this.key = key;
    this.topicId = id;
    this.version = version;
    this.generation++;
    this.controller.abort();
    this.controller = new AbortController();
    this.fetching = false;
    this.topic = null;
    this.connections = { items: [], nextCursor: null };
    this.paginated = false;
    this.error = '';
    if (id && !this.session.offline) void this.refresh(true);
  }
  async refresh(reset = true) {
    if (!this.topicId || this.session.offline || this.fetching) return;
    const generation = this.generation,
      id = this.topicId,
      signal = this.controller.signal;
    this.fetching = true;
    if (reset) this.loading = true;
    try {
      const [topic, connections] = await Promise.all([
        this.client.topic(id, this.version, signal),
        reset || !this.paginated
          ? this.client.connections(id, undefined, signal)
          : Promise.resolve(null)
      ]);
      if (generation !== this.generation) return;
      this.topic = topic;
      if (connections && (reset || !this.paginated)) {
        this.connections = connections;
        if (reset) this.paginated = false;
      }
      this.error = '';
    } catch (error) {
      if (generation === this.generation && !signal.aborted)
        this.error =
          error instanceof Error ? error.message : 'Saved connections could not be read.';
    } finally {
      if (generation === this.generation) {
        this.fetching = false;
        this.loading = false;
      }
    }
  }
  async more() {
    const id = this.topicId,
      cursor = this.connections.nextCursor,
      generation = this.generation;
    if (!id || !cursor || this.loading || this.session.offline) return;
    this.loading = true;
    try {
      const page = await this.client.connections(id, cursor, this.controller.signal);
      if (generation === this.generation) {
        this.connections = {
          items: [
            ...this.connections.items,
            ...page.items.filter(
              (item) => !this.connections.items.some((old) => old.id === item.id)
            )
          ],
          nextCursor: page.nextCursor
        };
        this.paginated = true;
      }
    } catch (error) {
      if (generation === this.generation) this.report(error);
    } finally {
      if (generation === this.generation) this.loading = false;
    }
  }
  private current(): Location | null {
    if (this.retained)
      return { kind: 'topic', id: this.retained.id, version: this.retained.selectedAnswer?.id };
    const e = this.session.exploration;
    return e ? { kind: 'exploration', id: e.id, view: structuredClone(e.view) } : null;
  }
  private remember() {
    const current = this.current();
    if (current) this.history = [...this.history.slice(-49), current];
  }
  report(error: unknown) {
    this.error = error instanceof Error ? error.message : 'The library request did not finish.';
  }
  cancelPending() {
    this.transition++;
  }
  async follow(id: string) {
    const transition = ++this.transition;
    const link = this.connections.items.find((item) => item.id === id);
    if (!link || !this.topicId || this.session.offline) return;
    const generation = this.generation,
      target = link.fromTopicId === this.topicId ? link.toTopicId : link.fromTopicId;
    try {
      await this.session.flushView();
      const topic = await this.client.topic(target);
      const version = link.passages.find((p) =>
        topic.versions.some((v) => v.id === p.answerVersionId)
      )?.answerVersionId;
      const places = await this.client.appearances(target, undefined, version);
      if (generation !== this.generation || transition !== this.transition) return;
      this.remember();
      this.onNavigate();
      if (places.items[0]) {
        const place = places.items[0];
        this.retained = null;
        await this.session.load(place.explorationId);
        if (transition !== this.transition) return;
        this.session.setView({ selectedNodeId: place.appearanceId, mobilePane: 'reader' });
      } else {
        const retained = version ? await this.client.topic(target, version) : topic;
        if (transition !== this.transition) return;
        this.retained = retained;
        this.track(target, this.retained.selectedAnswer?.id);
      }
    } catch (error) {
      if (transition === this.transition && generation === this.generation) this.report(error);
    }
  }
  async back() {
    const transition = ++this.transition;
    const place = this.history.at(-1);
    if (!place) return;
    try {
      await this.session.flushView();
      if (transition !== this.transition) return;
      this.history = this.history.slice(0, -1);
      this.onNavigate();
      if (place.kind === 'topic') {
        const topic = await this.client.topic(place.id, place.version);
        if (transition !== this.transition) return;
        this.retained = topic;
        this.track(place.id, place.version);
      } else {
        this.retained = null;
        await this.session.load(place.id);
        if (transition !== this.transition) return;
        this.session.setView(place.view);
      }
    } catch (error) {
      if (transition === this.transition) this.report(error);
    }
  }
  async choose(version: string) {
    const e = this.session.exploration;
    if (!e || this.session.readOnly) return;
    const id = e.view.selectedNodeId;
    try {
      await this.session.mutate(
        `/api/appearances/${id}/answer`,
        'PATCH',
        { appearanceId: id, answerVersionId: version },
        e.revision
      );
      await this.refresh(true);
    } catch (error) {
      this.report(error);
    }
  }
  async dismiss(id: string) {
    try {
      await this.session.mutate(`/api/connections/${id}`, 'PATCH', { connectionId: id }, null);
      this.connections = {
        ...this.connections,
        items: this.connections.items.filter((c) => c.id !== id)
      };
      await this.refresh(true);
    } catch (error) {
      this.report(error);
    }
  }
  async listTopics(more = false) {
    const generation = ++this.catalogGeneration;
    if (this.session.offline) {
      this.error = 'Reconnect to browse saved topics outside this device’s cached explorations.';
      return;
    }
    try {
      const page = await this.client.topics(
        this.search,
        more ? (this.catalog.nextCursor ?? undefined) : undefined
      );
      if (generation !== this.catalogGeneration) return;
      this.catalog = {
        items: more ? [...this.catalog.items, ...page.items] : page.items,
        nextCursor: page.nextCursor
      };
    } catch (error) {
      if (generation === this.catalogGeneration) this.report(error);
    }
  }
  searchTopics(value: string) {
    this.search = value;
    if (this.searchTimer) clearTimeout(this.searchTimer);
    this.searchTimer = setTimeout(() => {
      void this.listTopics();
    }, 300);
  }
  async openTopic(id: string, version?: string) {
    const transition = ++this.transition;
    try {
      const topic = await this.client.topic(id, version);
      await this.session.flushView();
      if (transition !== this.transition) return;
      this.remember();
      this.retained = topic;
      this.showCatalog = false;
      this.onNavigate();
      this.track(id, topic.selectedAnswer?.id);
    } catch (error) {
      if (transition === this.transition) this.report(error);
    }
  }
  async readVersion(version: string) {
    if (!this.retained) return;
    const id = this.retained.id,
      transition = ++this.transition;
    try {
      const topic = await this.client.topic(id, version);
      if (transition !== this.transition || this.retained?.id !== id) return;
      this.retained = topic;
      this.track(id, version);
    } catch (error) {
      if (transition === this.transition) this.report(error);
    }
  }
  async startRetained() {
    if (!this.retained || this.session.readOnly) return;
    const transition = ++this.transition;
    try {
      const topic = this.retained;
      this.onNavigate();
      await this.session.mutate(
        `/api/topics/${topic.id}/explorations`,
        'POST',
        { topicId: topic.id, answerVersionId: topic.selectedAnswer?.id ?? null },
        null
      );
      if (transition === this.transition && this.retained?.id === topic.id) this.retained = null;
    } catch (error) {
      if (transition === this.transition) this.report(error);
    }
  }
  reset() {
    this.transition++;
    this.catalogGeneration++;
    this.catalog = { items: [], nextCursor: null };
    this.search = '';
    this.loading = false;
    if (this.searchTimer) clearTimeout(this.searchTimer);
    this.history = [];
    this.retained = null;
    this.showCatalog = false;
    this.track(null);
    this.error = '';
  }
  dispose() {
    this.controller.abort();
    if (this.interval) clearInterval(this.interval);
    if (this.searchTimer) clearTimeout(this.searchTimer);
  }
}
