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
  import { createSession } from '#lib/knowledge/session.svelte.ts';
  import { createResearchSession } from '#lib/knowledge/server-session.svelte.ts';
  import { createLibraryClient, LibraryError } from '#lib/knowledge/library-client.ts';
  import { createLibraryCache } from '#lib/knowledge/cache.ts';
  import { migrateLegacy } from '#lib/knowledge/migration.ts';
  import {
    createExplorationRepository,
    createAutosave,
    readBrowserOriginals,
    type SaveStatus
  } from '#lib/knowledge/persistence.ts';
  import { createDemoExploration } from '#lib/knowledge/demo.ts';
  import type { Exploration, SavedExploration } from '#lib/knowledge/types.ts';
  let saved = $state<SavedExploration[]>([]),
    demos: Exploration[] = [];
  let demoMode = $state(false),
    demoStatus = $state<SaveStatus>('saved'),
    saveMessage = $state(''),
    error = $state(''),
    showLibrary = $state(false),
    switching = $state(false),
    mounted = $state(false),
    migrationMessage = $state(''),
    libraryLoaded = $state(false),
    nextCursor = $state<string | null>(null);
  const repository = createExplorationRepository(undefined, (message) => {
    error = message;
  });
  const cache = createLibraryCache();
  const client = createLibraryClient((...args) => fetch(...args));
  let channel: BroadcastChannel | null = null,
    lastAcknowledged = '';
  const research = createResearchSession({
    client,
    cache,
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
  const navigation = new LibraryNavigation(client, research, () => {
    demoMode = false;
    showLibrary = false;
  });
  let saver = createAutosave(repository, (next, message) => {
    demoStatus = next;
    saveMessage = message ?? '';
    if (next === 'saved') {
      void reloadSaved(false);
      channel?.postMessage({ demo: true });
    }
  });
  const demo = createSession({
    research: async () => {
      throw new Error('Choose a research exploration to request research.');
    },
    onChange(value) {
      if (value.mode === 'demo') saver.enqueue(value);
    }
  });
  const session = $derived(demoMode ? demo : research);
  const activeTopicId = $derived(
    demoMode
      ? null
      : (navigation.retained?.id ??
          research.exploration?.nodes[research.exploration.view.selectedNodeId]?.topicId ??
          null)
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
    demoMode
      ? demoStatus
      : research.conflict
        ? 'conflict'
        : research.viewError
          ? 'failed'
          : research.busy || research.recovering || research.viewSaving
            ? 'saving'
            : 'saved'
  );
  async function reloadSaved(server = true, append = false) {
    demos = (await repository.list()).filter((item) => item.mode === 'demo');
    let remote: SavedExploration[] = saved.filter((item) => item.mode !== 'demo');
    if (server && research.library) {
      if (research.offline)
        remote = (await cache.snapshots(research.library)).map((snapshot) => ({
          ...snapshot.exploration,
          mode: 'research',
          topicCount: snapshot.appearances.length
        }));
      else {
        const page = await client.explorations(append ? (nextCursor ?? undefined) : undefined);
        nextCursor = page.nextCursor;
        remote = [
          ...(append ? remote : []),
          ...page.items.map((item) => ({ ...item, mode: 'research' as const }))
        ];
      }
    }
    saved = [
      ...remote,
      ...demos.map((item) => ({ ...item, topicCount: Object.keys(item.nodes).length }))
    ].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  }
  let connecting: Promise<void> | null = null;
  function ensureLibrary() {
    return (connecting ??= (async () => {
      await research.connect();
      libraryLoaded = !!research.library;
      if (!research.offline && research.library) {
        const result = await migrateLegacy(repository, client, cache, {
          onProgress: (message) => {
            migrationMessage = message;
          }
        });
        migrationMessage = result.failed
          ? `${result.failed} browser copies could not be imported. ${result.messages.join(' ')}`
          : result.imported
            ? `${result.imported} browser copies imported. Originals remain on this device.`
            : result.messages.join(' ');
      }
      await reloadSaved();
    })().finally(() => {
      connecting = null;
    }));
  }
  async function beforeSwitch() {
    if (switching) return false;
    switching = true;
    try {
      if (demoMode) await saver.flush();
      else await research.flushView();
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
      demo.dispose();
      research.dispose();
      demoMode = false;
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
      const local = demos.find((item) => item.id === id);
      if (local) {
        research.dispose();
        demoMode = true;
        demo.load(local);
      } else {
        demo.dispose();
        demoMode = false;
        if (!libraryLoaded) await ensureLibrary();
        await research.load(id);
      }
      showLibrary = false;
      error = '';
    } catch (e) {
      error =
        e instanceof Error
          ? e.message
          : 'This saved exploration could not be read. Its original data is still stored.';
    } finally {
      switching = false;
    }
  }
  async function startDemo() {
    if (!(await beforeSwitch())) return;
    navigation.reset();
    research.conflict = false;
    try {
      research.dispose();
      demoMode = true;
      demo.load(createDemoExploration());
      saver.enqueue(demo.exploration!);
      showLibrary = false;
      error = '';
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
        const id = research.exploration?.id;
        research.dispose();
        research.exploration = null;
        research.conflict = false;
        if (id && research.library) await cache.removeSnapshot(research.library, id);
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
    session.dispose();
    session.exploration = null;
    showLibrary = false;
    switching = false;
    await reloadSaved(libraryLoaded && !demoMode);
  }
  async function openLibrary() {
    navigation.cancelPending();
    showLibrary = !showLibrary;
    if (!showLibrary || demoMode) return;
    try {
      await ensureLibrary();
    } catch (e) {
      error =
        e instanceof Error ? e.message : 'Open your private access link to view the saved library.';
    }
  }
  async function remove(id: string) {
    try {
      const local = demos.find((item) => item.id === id);
      if (local) {
        if (demo.exploration?.id === id) {
          demo.dispose();
          demo.exploration = null;
        }
        await repository.remove(id);
      } else {
        const item = saved.find((item) => item.id === id);
        if (!item) return;
        await research.mutate(
          `/api/explorations/${id}`,
          'DELETE',
          { explorationId: id },
          item.revision
        );
        await cache.removeSnapshot(research.library!, id);
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
      }
      await reloadSaved();
    } catch (e) {
      if (e instanceof LibraryError && e.code === 'REVISION_CONFLICT') await reloadSaved();
      error = e instanceof Error ? e.message : 'This exploration could not be deleted.';
    }
  }
  async function showTopics() {
    navigation.cancelPending();
    try {
      if (!libraryLoaded) await ensureLibrary();
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
      migrationMessage =
        'Server library cleared. Offline browser originals and retained backups require separate clearing.';
      channel?.postMessage({ purged: true });
    } catch (e) {
      error = e instanceof Error ? e.message : 'The library could not be cleared.';
    }
  }
  async function restoreCopies() {
    try {
      const result = await migrateLegacy(repository, client, cache, { explicitRestore: true });
      migrationMessage = `${result.imported} browser copies restored. Originals remain on this device. ${result.messages.join(' ')}`;
      await reloadSaved();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Browser copies could not be restored.';
    }
  }
  async function exportOriginals() {
    const originals = await readBrowserOriginals();
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(originals, null, 2)], { type: 'application/json' })
    );
    const link = document.createElement('a');
    link.href = url;
    link.download = 'branch-browser-originals.json';
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  async function clearLocalCopies() {
    try {
      await research.flushView();
      research.dispose();
      research.exploration = null;
      navigation.reset();
      for (const item of await repository.list())
        if (item.mode === 'research') await repository.remove(item.id);
      await cache.clear();
      saved = saved.filter((item) => item.mode === 'demo');
      channel?.postMessage({ localCopiesCleared: true });
      await reloadSaved(false);
      migrationMessage =
        'Research copies cleared on this device. The server library, other devices, demos, and retained backups are separate.';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Browser copies could not be cleared.';
    }
  }
  onMount(() => {
    mounted = true;
    navigation.start();
    void (async () => {
      await reloadSaved(false);
      const originals = await repository.list();
      if ((await cache.lastLibrary()) || originals.some((item) => item.mode === 'research'))
        await ensureLibrary();
    })().catch((e) => {
      error =
        e instanceof Error
          ? e.message
          : 'The library could not be opened. Browser originals are preserved.';
    });
    if (typeof BroadcastChannel !== 'undefined') {
      channel = new BroadcastChannel('branch-explorations');
      channel.onmessage = async (event) => {
        if (demoMode && !event.data?.demo) return;
        if (event.data?.localCopiesCleared) {
          research.dispose();
          research.exploration = null;
          navigation.reset();
          saved = saved.filter((item) => item.mode === 'demo');
          return;
        }
        if (event.data?.purged) {
          research.dispose();
          research.exploration = null;
          navigation.reset();
          await research.connect();
          await reloadSaved();
          return;
        }
        const info = research.library;
        if (event.data?.demo) {
          await reloadSaved(false);
          return;
        }
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
      };
    }
    return () => {
      navigation.dispose();
      research.dispose();
      demo.dispose();
      saver.dispose();
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
{#snippet savedConnections()}{#if activeTopicId && !demoMode}<TopicConnections
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
            ? research.offline && !demoMode
              ? 'Offline saved copy'
              : 'All changes saved'
            : status === 'saving'
              ? 'Saving…'
              : status === 'conflict'
                ? 'Changed in another tab'
                : 'Not saved'}</span
        ><button class="quiet-button" onclick={home}>+ New exploration</button
        >{/if}{#if !demoMode}<button
          class="quiet-button saved-topics-button"
          disabled={!mounted}
          onclick={showTopics}>Saved topics</button
        >{/if}<button
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
  {#if research.notice && !demoMode && !research.exploration}<div
      class="workspace-notice"
      role="status"
    >
      {research.notice}
    </div>{/if}
  {#if session.exploration?.mode === 'demo'}
    <div class="demo-banner" role="note" aria-label="Demo content">
      <strong>Demo content</strong>
      <span
        >Prewritten explanations. Explore the branches and save your own questions without research.</span
      >
    </div>
  {/if}
  {#if navigation.history.length && !demoMode}<div class="return-location">
      <button
        class="quiet-button"
        onclick={() => {
          void navigation.back();
        }}>Back to previous topic</button
      >
    </div>{/if}
  {#if navigation.error && !demoMode}<div class="workspace-notice" role="alert">
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
  {#if research.offline && !demoMode}<div class="workspace-notice" role="status">
      Offline saved copy · Read-only until you reconnect. <button
        class="quiet-button"
        onclick={() => {
          void ensureLibrary().catch((e) => (error = e.message));
        }}>Reconnect library</button
      >
    </div>{/if}
  {#if migrationMessage && !demoMode}<div class="workspace-notice" role="status">
      {migrationMessage}
    </div>{/if}
  {#if status === 'failed' || status === 'conflict'}<div class="save-alert" role="alert">
      <p>{demoMode || status === 'conflict' ? saveMessage : research.viewError}</p>
      {#if status === 'conflict' && session.exploration}<button
          class="quiet-button"
          disabled={switching}
          onclick={reloadConflict}>Reload saved version</button
        >{:else}<button
          class="quiet-button"
          onclick={() => {
            void (demoMode ? saver.flush() : research.retryView()).catch(() => {});
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
      {#if !demoMode}<LibraryStatus
          library={research.library}
          offline={research.offline}
          busy={research.busy || research.recovering}
          onPurge={(confirmation) => {
            void purge(confirmation);
          }}
          onRestore={() => {
            void restoreCopies();
          }}
          onExport={() => {
            void exportOriginals();
          }}
          onClearLocal={() => {
            void clearLocalCopies();
          }}
        />{/if}
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
  {#if navigation.showCatalog && !demoMode}<section class="topic-catalog" aria-label="Saved topics">
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
      >{#snippet readerExtras()}{#if !demoMode}{#if navigation.topic}<AnswerVersions
              topic={navigation.topic}
              readOnly={research.readOnly || research.busy}
              onChoose={(id) => {
                void navigation.choose(id);
              }}
            />{/if}{@render savedConnections()}{/if}{/snippet}</Workspace
    >{:else}<main class="home-main">
      <div class="home-grid">
        <TopicStart onStart={start} onDemo={startDemo} disabled={switching || !mounted} />
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
        <ExplorationList explorations={saved} onResume={resume} onDelete={remove} />
        <div class="home-promise">
          <span>01 <strong>Start anywhere.</strong></span><span
            >02 <strong>Understand the details.</strong></span
          ><span>03 <strong>See where it takes you.</strong></span>
        </div>
      </div>
      <footer class="home-footer">
        <span>Made for the beautifully curious.</span><span
          >Private saved library · Demos stay on this device</span
        >
      </footer>
    </main>{/if}
</div>
