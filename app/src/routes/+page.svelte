<script lang="ts">
  import { css, cva } from '#styled/css';
  import { button } from '#styled/recipes';
  import AppHeader from '#lib/components/AppHeader.svelte';
  import { onMount, untrack } from 'svelte';
  import TopicStart from '#lib/components/TopicStart.svelte';
  import StartIllustration from '#lib/components/StartIllustration.svelte';
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
  const shell = cva({
    base: { minH: 'viewport', position: 'relative' },
    variants: {
      exploring: {
        true: {
          display: 'flex',
          flexDirection: 'column',
          workspace: { h: 'viewport', overflow: 'hidden' }
        }
      }
    }
  });
  const styles = {
    notice: css({
      textStyle: 'body.sm',
      display: 'flex',
      flexWrap: 'wrap',
      alignItems: 'center',
      gap: '3',
      px: 'gutter',
      py: '3',
      bg: 'accent.subtle',
      borderBottomWidth: '1px',
      borderColor: 'border',
      flexShrink: '0',
      overflowWrap: 'anywhere'
    }),
    label: css({
      textStyle: 'label',
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'center',
      gap: '3',
      color: 'fg.muted'
    }),
    drawer: css({
      position: 'fixed',
      zIndex: '50',
      top: '24',
      bottom: '4',
      left: 'gutter',
      right: 'gutter',
      maxW: 'lg',
      ml: 'auto',
      overflowY: 'auto',
      bg: 'bg.surface',
      borderWidth: '1px',
      borderColor: 'border.strong',
      rounded: 'panel',
      shadow: 'lg',
      p: '5',
      display: 'flex',
      flexDirection: 'column',
      gap: '5'
    }),
    catalog: css({
      position: 'fixed',
      zIndex: '40',
      inset: 'gutter',
      bg: 'bg.surface',
      borderWidth: '1px',
      borderColor: 'border.strong',
      rounded: 'panel',
      p: 'section',
      overflowY: 'auto',
      display: 'flex',
      flexDirection: 'column',
      gap: '4',
      sm: { left: 'auto', w: '2xl' },
      maxW: 'full'
    }),
    input: css({
      w: 'full',
      minW: '0',
      minH: 'control',
      borderWidth: '1px',
      borderColor: 'border.strong',
      rounded: 'panel',
      bg: 'bg.surface',
      px: '3',
      py: '2'
    }),
    catalogTopic: css({
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'start',
      textAlign: 'left',
      gap: '2',
      p: '4',
      rounded: 'control',
      borderWidth: '1px',
      borderColor: 'border',
      bg: 'bg',
      overflowWrap: 'anywhere',
      _hover: { bg: 'accent.subtle', borderColor: 'accent' }
    })
  };
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
      ? navigation.retained.question + ' — Trellis'
      : session.exploration
        ? session.exploration.originalPrompt + ' — Trellis'
        : 'Trellis — follow your curiosity'}</title
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
<div class={shell({ exploring: !!session.exploration || !!navigation.retained })}>
  <AppHeader
    exploring={!!session.exploration}
    saveState={status === 'saved' && research.offline ? 'offline' : status}
    disabled={!mounted}
    explorationCount={saved.length}
    onHome={home}
    onSavedTopics={showTopics}
    onExplorations={openLibrary}
  />
  {#if research.notice && !research.exploration}<div class={styles.notice} role="status">
      {research.notice}
    </div>{/if}
  {#if navigation.history.length}<div
      class={css({
        px: 'gutter',
        py: '2',
        flexShrink: '0',
        bg: 'bg.surface',
        borderBottomWidth: '1px',
        borderColor: 'border'
      })}
    >
      <button
        class={button({ visual: 'quiet' })}
        onclick={() => {
          void navigation.back();
        }}>Back to previous topic</button
      >
    </div>{/if}
  {#if navigation.error}<div class={styles.notice} role="alert">
      {navigation.error}<button
        class={button({ visual: 'icon' })}
        aria-label="Dismiss library error"
        onclick={() => (navigation.error = '')}>×</button
      >
    </div>{/if}
  {#if error}<div class={styles.notice} role="alert">
      {error}<button
        class={button({ visual: 'icon' })}
        aria-label="Dismiss error"
        onclick={() => (error = '')}>×</button
      >
    </div>{/if}
  {#if research.offline}<div class={styles.notice} role="status">
      Server disconnected. Reconnect to load or change saved content. <button
        class={button({ visual: 'quiet' })}
        onclick={() => {
          void ensureLibrary().catch((e) => (error = e.message));
        }}>Reconnect library</button
      >
    </div>{/if}
  {#if status === 'failed' || status === 'conflict'}<div class={styles.notice} role="alert">
      <p>
        {status === 'conflict'
          ? saveMessage || 'This exploration changed. Reload before continuing.'
          : research.viewError}
      </p>
      {#if status === 'conflict' && session.exploration}<button
          class={button({ visual: 'quiet' })}
          disabled={switching}
          onclick={reloadConflict}>Reload saved version</button
        >{:else}<button
          class={button({ visual: 'quiet' })}
          onclick={() => {
            void research.retryView().catch(() => {});
          }}>Retry saving</button
        >{/if}
    </div>{/if}
  {#if showLibrary}<div class={styles.drawer} role="region" aria-label="Your explorations drawer">
      <div class={styles.label}>
        <h2 class={css({ fontFamily: 'heading', fontSize: '2xl', color: 'fg' })}>
          Your explorations
        </h2>
        <button
          class={button({ visual: 'icon' })}
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
          class={button({ visual: 'quiet' })}
          onclick={() => {
            void reloadSaved(true, true);
          }}>More explorations</button
        >{/if}{#if !saved.length}<p class={css({ textStyle: 'body.sm', color: 'fg.muted' })}>
          Your explorations will find a home here.
        </p>{/if}
    </div>{/if}
  {#if navigation.showCatalog}<section class={styles.catalog} aria-label="Saved topics">
      <div class={styles.label}>
        <h2 class={css({ fontFamily: 'heading', fontSize: '2xl', color: 'fg' })}>Saved topics</h2>
        <button
          class={button({ visual: 'icon' })}
          aria-label="Close saved topics"
          onclick={() => (navigation.showCatalog = false)}>×</button
        >
      </div>
      <label for="topic-search">Search saved topics</label><input
        id="topic-search"
        class={styles.input}
        maxlength="500"
        value={navigation.search}
        oninput={(event) => navigation.searchTopics(event.currentTarget.value)}
      />{#each navigation.catalog.items as topic (topic.id)}<button
          class={styles.catalogTopic}
          aria-label={'Read saved topic ' + topic.question}
          onclick={() => {
            void navigation.openTopic(topic.id);
          }}
          ><strong class={css({ textStyle: 'body.sm' })}>{topic.question}</strong><span
            class={css({ textStyle: 'caption', color: 'fg.muted' })}
            >{topic.answerVersionCount} saved answers</span
          ></button
        >{/each}{#if navigation.catalog.nextCursor}<button
          class={button({ visual: 'quiet' })}
          onclick={() => {
            void navigation.listTopics(true);
          }}>More saved topics</button
        >{/if}
    </section>{/if}
  <div class={css({ display: 'contents' })} inert={switching}>
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
      >{:else}<main class={css({ maxW: '7xl', mx: 'auto', px: 'gutter', w: 'full' })}>
        <div
          class={css({
            display: 'grid',
            gap: '8',
            alignItems: 'center',
            py: '6',
            workspace: { gridTemplateColumns: '1.2fr 1fr', minH: '96', py: '10' }
          })}
        >
          <TopicStart onStart={start} disabled={switching || !mounted} />
          <StartIllustration />
        </div>
        <div class={css({ display: 'grid', gap: '8', py: '6' })}>
          {#if !showLibrary}<ExplorationList
              explorations={saved}
              onResume={resume}
              onDelete={remove}
            />{/if}
          <div
            class={css({
              display: 'flex',
              flexWrap: 'wrap',
              gap: '6',
              color: 'fg.muted',
              textStyle: 'body.sm'
            })}
          >
            <span>01 <strong>Start anywhere.</strong></span><span
              >02 <strong>Understand the details.</strong></span
            ><span>03 <strong>See where it takes you.</strong></span>
          </div>
        </div>
        <footer
          class={css({
            display: 'flex',
            flexWrap: 'wrap',
            justifyContent: 'space-between',
            gap: '3',
            textStyle: 'caption',
            color: 'fg.muted',
            borderTopWidth: '1px',
            borderColor: 'border',
            py: '6'
          })}
        >
          <span>Made for the beautifully curious.</span><span
            >Private saved library · Stored on your server</span
          >
        </footer>
      </main>{/if}
  </div>
</div>
