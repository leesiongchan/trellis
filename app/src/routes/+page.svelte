<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import '@xyflow/svelte/dist/style.css';
  import TopicStart from '#lib/components/TopicStart.svelte';
  import ExplorationList from '#lib/components/ExplorationList.svelte';
  import Workspace from '#lib/components/Workspace.svelte';
  import TopicConnections from '#lib/components/TopicConnections.svelte';
  import AnswerVersions from '#lib/components/AnswerVersions.svelte';
  import RetainedTopic from '#lib/components/RetainedTopic.svelte';
  import LibraryStatus from '#lib/components/LibraryStatus.svelte';
  import { LibraryNavigation } from '#lib/knowledge/library-navigation.svelte.ts';
  import { createResearchSession } from '#lib/knowledge/server-session.svelte.ts';
  import { createLibraryClient, LibraryError } from '#lib/knowledge/library-client.ts';
  import type { SavedExploration } from '#lib/knowledge/types.ts';
  let saved = $state<SavedExploration[]>([]);
  let saveMessage = $state(''),
    error = $state(''),
    showLibrary = $state(false),
    switching = $state(false),
    mounted = $state(false),
    libraryLoaded = $state(false),
    nextCursor = $state<string | null>(null);
  const client = createLibraryClient((...args) => fetch(...args));
  let channel: BroadcastChannel | null = null,
    lastAcknowledged = '';
  const research = createResearchSession({
    client,
    onChange(value) {
      const key = value.id + ':' + value.revision;
      if (lastAcknowledged !== key) {
        lastAcknowledged = key;
        channel?.postMessage({
          id: value.id,
          revision: value.revision,
          libraryId: research.library?.libraryId,
          epoch: research.library?.libraryEpoch
        });
      }
    }
  });
  const session = research;
  const navigation = new LibraryNavigation(client, research, () => {
    showLibrary = false;
  });
  const activeTopicId = $derived(
    navigation.retained?.id ??
      research.exploration?.nodes[research.exploration.view.selectedNodeId]?.topicId ??
      null
  );
  $effect(() => {
    const topicId = activeTopicId,
      version =
        navigation.retained?.selectedAnswer?.id ??
        research.exploration?.nodes[research.exploration.view.selectedNodeId]?.answerVersionId ??
        undefined;
    const offline = research.offline;
    untrack(() => {
      void offline;
      navigation.track(topicId, version);
    });
  });
  const status = $derived(
    research.conflict
      ? 'conflict'
      : research.viewError
        ? 'failed'
        : research.busy || research.recovering || research.viewSaving
          ? 'saving'
          : 'saved'
  );
  async function reloadSaved(server = true, append = false) {
    if (!server || !research.library || research.offline) return;
    const page = await client.explorations(append ? (nextCursor ?? undefined) : undefined);
    nextCursor = page.nextCursor;
    saved = [...(append ? saved : []), ...page.items];
  }
  let connecting: Promise<void> | null = null;
  function ensureLibrary() {
    return (connecting ??= (async () => {
      await research.connect();
      libraryLoaded = !!research.library;
      await reloadSaved();
    })().finally(() => {
      connecting = null;
    }));
  }
  async function beforeSwitch() {
    if (switching) return false;
    switching = true;
    try {
      await research.flushView();
      return true;
    } catch {
      switching = false;
      return false;
    }
  }
  async function start(question: string) {
    if (!(await beforeSwitch())) return;
    navigation.reset();
    research.conflict = false;
    try {
      research.dispose();
      showLibrary = false;
      error = '';
      await ensureLibrary();
      switching = false;
      await research.start(question);
      await reloadSaved();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not start this exploration.';
    } finally {
      switching = false;
    }
  }
  async function resume(id: string) {
    if (!(await beforeSwitch())) return;
    navigation.reset();
    research.conflict = false;
    try {
      if (!libraryLoaded || research.offline) await ensureLibrary();
      await research.load(id);
      showLibrary = false;
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'This exploration could not be opened.';
    } finally {
      switching = false;
    }
  }
  async function reloadConflict() {
    if (!research.exploration || switching) return;
    switching = true;
    try {
      await research.load(research.exploration.id);
      error = '';
      await reloadSaved();
    } catch (e) {
      if (e instanceof LibraryError && e.status === 404) {
        research.dispose();
        research.exploration = null;
        research.conflict = false;
        await reloadSaved();
        error = 'This exploration was deleted in another tab. Start a new exploration to continue.';
      } else error = 'The saved version could not be read. Your current work is still open.';
    } finally {
      switching = false;
    }
  }
  async function home() {
    if (!(await beforeSwitch())) return;
    navigation.reset();
    research.conflict = false;
    research.dispose();
    research.exploration = null;
    showLibrary = false;
    switching = false;
    try {
      await reloadSaved();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load saved explorations.';
    }
  }
  async function openLibrary() {
    navigation.cancelPending();
    showLibrary = !showLibrary;
    if (!showLibrary) return;
    try {
      await ensureLibrary();
    } catch (e) {
      error =
        e instanceof Error ? e.message : 'Open your private access link to view the saved library.';
    }
  }
  async function remove(id: string) {
    try {
      const item = saved.find((item) => item.id === id);
      if (!item) return;
      await research.mutate(
        `/api/explorations/${id}`,
        'DELETE',
        { explorationId: id },
        item.revision
      );
      if (research.exploration?.id === id) {
        research.dispose();
        research.exploration = null;
      }
      channel?.postMessage({
        id,
        revision: -1,
        libraryId: research.library?.libraryId,
        epoch: research.library?.libraryEpoch
      });
      await reloadSaved();
    } catch (e) {
      if (e instanceof LibraryError && e.code === 'REVISION_CONFLICT') await reloadSaved();
      error = e instanceof Error ? e.message : 'This exploration could not be deleted.';
    }
  }
  async function showTopics() {
    navigation.cancelPending();
    try {
      if (!libraryLoaded || research.offline) await ensureLibrary();
      showLibrary = false;
      navigation.showCatalog = !navigation.showCatalog;
      if (navigation.showCatalog) await navigation.listTopics();
    } catch (e) {
      navigation.report(e);
    }
  }
  async function purge(confirmation: string) {
    try {
      await research.mutate('/api/library/purge', 'POST', { confirmation }, null);
      research.dispose();
      research.exploration = null;
      navigation.reset();
      await research.connect();
      await reloadSaved();
      research.notice = 'Server library cleared. Retained backups are managed separately.';
      channel?.postMessage({ purged: true });
    } catch (e) {
      error = e instanceof Error ? e.message : 'The library could not be cleared.';
    }
  }
  onMount(() => {
    mounted = true;
    navigation.start();
    void ensureLibrary().catch((e) => {
      error = e instanceof Error ? e.message : 'The server library could not be opened.';
    });
    if (typeof BroadcastChannel !== 'undefined') {
      channel = new BroadcastChannel('branch-explorations');
      channel.onmessage = (event) => {
        void (async () => {
          if (event.data?.purged) {
            research.dispose();
            research.exploration = null;
            navigation.reset();
            await research.connect();
            await reloadSaved();
            return;
          }
          const info = research.library;
          if (
            !info ||
            event.data?.libraryId !== info.libraryId ||
            event.data?.epoch !== info.libraryEpoch
          )
            return;
          await reloadSaved();
          if (
            event.data?.id === research.exploration?.id &&
            event.data?.revision !== research.exploration?.revision
          ) {
            research.conflict = true;
            saveMessage =
              'This exploration changed in another tab. Reload the saved version before making more changes.';
          }
        })().catch((e) => {
          error = e instanceof Error ? e.message : 'The server library could not be refreshed.';
        });
      };
    }
    return () => {
      navigation.dispose();
      research.dispose();
      channel?.close();
    };
  });
</script>

<svelte:head
  ><title
    >{navigation.retained
      ? navigation.retained.question + ' — Branch'
      : session.exploration
        ? session.exploration.originalPrompt + ' — Branch'
        : 'Branch — follow your curiosity'}</title
  ><meta
    name="description"
    content="Start with a question. Explore detailed explanations and follow an ever-growing tree of connected topics."
  /></svelte:head
>
{#snippet savedConnections()}{#if activeTopicId}<TopicConnections
      connections={navigation.connections}
      topicId={activeTopicId}
      loading={navigation.loading}
      discovery={research.library?.discovery ?? 'unavailable'}
      offline={research.offline}
      onFollow={(id) => {
        void navigation.follow(id);
      }}
      onDismiss={(id) => {
        void navigation.dismiss(id);
      }}
      onMore={() => {
        void navigation.more();
      }}
      onRefresh={() => {
        void navigation.refresh(true);
      }}
    />{/if}{/snippet}
<div class="app-shell" class:exploring={!!session.exploration || !!navigation.retained}>
  <header class="app-header">
    <button class="wordmark" aria-label="Branch home" onclick={home}
      ><span class="logo-mark" aria-hidden="true">⌘</span>branch<span class="logo-dot">.</span
      ></button
    ><span class="header-divider"></span><span class="header-tagline"
      >Room for a little wonder.</span
    >
    <div class="header-actions">
      {#if session.exploration}<span
          class="save-status"
          class:save-failed={status === 'failed' || status === 'conflict'}
          ><span class="green-dot"></span>{status === 'saved'
            ? research.offline
              ? 'Server disconnected'
              : 'All changes saved'
            : status === 'saving'
              ? 'Saving…'
              : status === 'conflict'
                ? 'Changed in another tab'
                : 'Not saved'}</span
        ><button class="quiet-button" onclick={home}>+ New exploration</button>{/if}<button
        class="quiet-button saved-topics-button"
        disabled={!mounted}
        onclick={showTopics}>Saved topics</button
      ><button
        class="library-button"
        aria-label="Your explorations"
        disabled={!mounted}
        onclick={openLibrary}
        ><span aria-hidden="true">▤</span><span>Your explorations</span>{#if saved.length}<span
            class="library-count">{saved.length}</span
          >{/if}</button
      >
    </div>
  </header>
  {#if research.notice && !research.exploration}<div class="workspace-notice" role="status">
      {research.notice}
    </div>{/if}
  {#if navigation.history.length}<div class="return-location">
      <button
        class="quiet-button"
        onclick={() => {
          void navigation.back();
        }}>Back to previous topic</button
      >
    </div>{/if}
  {#if navigation.error}<div class="workspace-notice" role="alert">
      {navigation.error}<button
        class="icon-button"
        aria-label="Dismiss library error"
        onclick={() => (navigation.error = '')}>×</button
      >
    </div>{/if}
  {#if error}<div class="workspace-notice" role="alert">
      {error}<button class="icon-button" aria-label="Dismiss error" onclick={() => (error = '')}
        >×</button
      >
    </div>{/if}
  {#if research.offline}<div class="workspace-notice" role="status">
      Server disconnected. Reconnect to load or change saved content. <button
        class="quiet-button"
        onclick={() => {
          void ensureLibrary().catch((e) => (error = e.message));
        }}>Reconnect library</button
      >
    </div>{/if}
  {#if status === 'failed' || status === 'conflict'}<div class="save-alert" role="alert">
      <p>
        {status === 'conflict'
          ? saveMessage || 'This exploration changed. Reload before continuing.'
          : research.viewError}
      </p>
      {#if status === 'conflict' && session.exploration}<button
          class="quiet-button"
          disabled={switching}
          onclick={reloadConflict}>Reload saved version</button
        >{:else}<button
          class="quiet-button"
          onclick={() => {
            void research.retryView().catch(() => {});
          }}>Retry saving</button
        >{/if}
    </div>{/if}
  {#if showLibrary}<div class="library-drawer">
      <div class="section-label">
        YOUR EXPLORATIONS<button
          class="icon-button"
          aria-label="Close explorations"
          onclick={() => (showLibrary = false)}>×</button
        >
      </div>
      <LibraryStatus
        library={research.library}
        offline={research.offline}
        busy={research.busy || research.recovering}
        onPurge={(confirmation) => {
          void purge(confirmation);
        }}
      />
      <ExplorationList
        explorations={saved}
        onResume={resume}
        onDelete={remove}
      />{#if nextCursor && !research.offline}<button
          class="quiet-button"
          onclick={() => {
            void reloadSaved(true, true);
          }}>More explorations</button
        >{/if}{#if !saved.length}<p class="empty-library">
          Your explorations will find a home here.
        </p>{/if}
    </div>{/if}
  {#if navigation.showCatalog}<section class="topic-catalog" aria-label="Saved topics">
      <div class="section-label">
        SAVED TOPICS<button
          class="icon-button"
          aria-label="Close saved topics"
          onclick={() => (navigation.showCatalog = false)}>×</button
        >
      </div>
      <label for="topic-search">Search saved topics</label><input
        id="topic-search"
        maxlength="500"
        value={navigation.search}
        oninput={(event) => navigation.searchTopics(event.currentTarget.value)}
      />{#each navigation.catalog.items as topic (topic.id)}<button
          class="catalog-topic"
          aria-label={'Read saved topic ' + topic.question}
          onclick={() => {
            void navigation.openTopic(topic.id);
          }}
          ><strong>{topic.question}</strong><span>{topic.answerVersionCount} saved answers</span
          ></button
        >{/each}{#if navigation.catalog.nextCursor}<button
          class="quiet-button"
          onclick={() => {
            void navigation.listTopics(true);
          }}>More saved topics</button
        >{/if}
    </section>{/if}
  {#if navigation.retained && mounted}<RetainedTopic
      topic={navigation.retained}
      disabled={research.offline || research.busy}
      onStartExploration={() => {
        void navigation.startRetained().then(() => reloadSaved());
      }}
      onReadVersion={(id) => {
        void navigation.readVersion(id);
      }}>{@render savedConnections()}</RetainedTopic
    >
  {:else if session.exploration && mounted}<Workspace {session}
      >{#snippet readerExtras()}{#if navigation.topic}<AnswerVersions
            topic={navigation.topic}
            readOnly={research.readOnly || research.busy}
            onChoose={(id) => {
              void navigation.choose(id);
            }}
          />{/if}{@render savedConnections()}{/snippet}</Workspace
    >{:else}<main class="home-main">
      <div class="home-grid">
        <TopicStart onStart={start} disabled={switching || !mounted} />
        <div class="tree-illustration" aria-hidden="true">
          <div class="illustration-eyebrow">A QUESTION IS JUST THE BEGINNING</div>
          <svg viewBox="0 0 480 440" fill="none"
            ><path
              d="M82 220H158M158 103V337M158 103H225M158 220H225M158 337H225M337 103H363M363 53V153M363 53H403M363 153H403M337 337H395"
              stroke="#b4c4aa"
              stroke-width="1.5"
            /><circle cx="82" cy="220" r="27" fill="#e5ecdb" stroke="#a7bd96" /><path
              d="M73 220H91M82 211V229"
              stroke="#637e50"
              stroke-width="1.5"
            /><rect
              x="225"
              y="72"
              width="112"
              height="62"
              rx="11"
              fill="#f0f3e8"
              stroke="#b8c7ad"
            /><rect
              x="225"
              y="189"
              width="137"
              height="62"
              rx="11"
              fill="#fffefa"
              stroke="#cbd4c2"
            /><rect
              x="225"
              y="306"
              width="112"
              height="62"
              rx="11"
              fill="#fffefa"
              stroke="#cbd4c2"
            /><circle cx="415" cy="53" r="12" fill="#edf2e5" stroke="#b6c7a6" /><circle
              cx="415"
              cy="153"
              r="12"
              fill="#edf2e5"
              stroke="#b6c7a6"
            /><circle cx="407" cy="337" r="12" fill="#edf2e5" stroke="#b6c7a6" /><text
              x="243"
              y="99"
              fill="#506746"
              font-size="11"
              font-family="system-ui">How it works</text
            ><text x="243" y="117" fill="#8a967e" font-size="9" font-family="system-ui"
              >Look a little closer ↗</text
            ><text x="243" y="217" fill="#506746" font-size="11" font-family="system-ui"
              >What makes it possible</text
            ><text x="243" y="235" fill="#8a967e" font-size="9" font-family="system-ui"
              >Find the foundations ↗</text
            ><text x="243" y="333" fill="#506746" font-size="11" font-family="system-ui"
              >Something unexpected</text
            ><text x="243" y="351" fill="#8a967e" font-size="9" font-family="system-ui"
              >Make a connection ↗</text
            ><circle cx="158" cy="220" r="3" fill="#91a57b" /></svg
          ><span class="illustration-caption">Every answer opens another door.</span>
        </div>
      </div>
      <div class="home-bottom">
        {#if !showLibrary}<ExplorationList
            explorations={saved}
            onResume={resume}
            onDelete={remove}
          />{/if}
        <div class="home-promise">
          <span>01 <strong>Start anywhere.</strong></span><span
            >02 <strong>Understand the details.</strong></span
          ><span>03 <strong>See where it takes you.</strong></span>
        </div>
      </div>
      <footer class="home-footer">
        <span>Made for the beautifully curious.</span><span
          >Private saved library · Stored on your server</span
        >
      </footer>
    </main>{/if}
</div>
