<script lang="ts">
  import { css, sva } from '#styled/css';
  import { button } from '#styled/recipes';
  import { MediaQuery } from 'svelte/reactivity';
  import type { Snippet } from 'svelte';
  import type { WorkspaceSession } from '#lib/knowledge/types.ts';
  import { ancestorPath, visibleTree } from '#lib/knowledge/tree.ts';
  import TopicTree from './TopicTree.svelte';
  import TopicReader from './TopicReader.svelte';
  let { session, readerExtras }: { session: WorkspaceSession; readerExtras?: Snippet } = $props();
  let error = $state('');
  const exploration = $derived(session.exploration!);
  const selected = $derived(exploration.nodes[exploration.view.selectedNodeId]);
  const path = $derived(ancestorPath(exploration, selected.id));
  const isHidden = $derived(!visibleTree(exploration).nodes.some((n) => n.id === selected.id));
  const wide = new MediaQuery('(min-width: 64rem)', true);
  const workspace = sva({
    slots: ['root', 'map', 'reader'],
    base: {
      root: {
        flex: '1',
        minH: '0',
        minW: '0',
        position: 'relative',
        overflow: 'hidden',
        display: 'grid',
        workspace: { gridTemplateColumns: 'minmax(0, .9fr) minmax(0, 1.1fr)' }
      },
      map: {
        position: 'absolute',
        inset: '0',
        minH: '0',
        minW: '0',
        workspace: { position: 'relative', inset: 'auto' }
      },
      reader: {
        position: 'absolute',
        inset: '0',
        minH: '0',
        minW: '0',
        overflowY: 'auto',
        workspace: { position: 'relative', inset: 'auto' }
      }
    },
    variants: {
      pane: {
        map: { reader: { display: 'none', workspace: { display: 'block' } } },
        reader: {
          map: {
            opacity: '0',
            pointerEvents: 'none',
            workspace: { opacity: '1', pointerEvents: 'auto' }
          }
        }
      }
    }
  });
  const classes = $derived(workspace({ pane: exploration.view.mobilePane }));
  const notice = css({
    textStyle: 'body.sm',
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    gap: '3',
    bg: 'accent.subtle',
    px: 'gutter',
    py: '3',
    overflowWrap: 'anywhere',
    flexShrink: '0'
  });
  function navigate(action: Promise<void>) {
    void action.catch(
      (e) => (error = e instanceof Error ? e.message : 'The library request did not finish.')
    );
  }
  function follow(question: string) {
    error = '';
    void session
      .addQuestion(selected.id, question)
      .catch((e) => (error = e instanceof Error ? e.message : 'Could not open this branch.'));
  }
  function reveal() {
    session.setView({
      expandedNodeIds: [
        ...new Set([...exploration.view.expandedNodeIds, ...path.slice(0, -1).map((n) => n.id)])
      ],
      mobilePane: 'map'
    });
  }
</script>

<div
  class={css({
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    gap: '2',
    px: 'gutter',
    py: '2',
    bg: 'bg.surface',
    borderBottomWidth: '1px',
    borderColor: 'border',
    flexShrink: '0'
  })}
>
  <nav
    class={css({
      display: 'flex',
      alignItems: 'center',
      gap: '1',
      flexGrow: '1',
      flexShrink: '1',
      flexBasis: '72',
      minW: '0',
      overflowX: 'auto',
      p: '1',
      scrollbarWidth: 'thin'
    })}
    aria-label="Topic path"
  >
    <button
      class={button({ visual: 'icon' })}
      aria-label="Return to root"
      onclick={() => navigate(session.select(exploration.rootId))}>⌂</button
    >{#each path as node, i (node.id)}{#if i}<span
          class={css({ color: 'fg.muted' })}
          aria-hidden="true">›</span
        >{/if}<button
        class={[
          button({ visual: 'quiet', size: 'sm' }),
          css({
            maxW: '48',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            display: 'block'
          })
        ]}
        aria-current={node.id === selected.id ? 'location' : undefined}
        onclick={() => navigate(session.select(node.id))}
        >{node.content?.title ?? node.question}</button
      >{/each}
  </nav>
  <div
    class={css({
      display: 'flex',
      workspace: { display: 'none' },
      bg: 'accent.subtle',
      p: '1',
      rounded: 'control',
      gap: '1'
    })}
    aria-label="Workspace view"
  >
    <button
      class={[
        button({ visual: 'quiet' }),
        css({ bg: exploration.view.mobilePane === 'map' ? 'bg.surface' : 'transparent' })
      ]}
      aria-pressed={exploration.view.mobilePane === 'map'}
      onclick={() => session.setView({ mobilePane: 'map' })}>Map</button
    ><button
      class={[
        button({ visual: 'quiet' }),
        css({ bg: exploration.view.mobilePane === 'reader' ? 'bg.surface' : 'transparent' })
      ]}
      aria-pressed={exploration.view.mobilePane === 'reader'}
      onclick={() => session.setView({ mobilePane: 'reader' })}>Read</button
    >
  </div>
  <button
    class={button({ visual: 'quiet', size: 'sm' })}
    onclick={() => session.setView({ expandedNodeIds: [] })}>Collapse branches</button
  >
</div>
{#if error || session.notice}<div class={notice} role="status">
    {error || session.notice}<button
      class={button({ visual: 'icon' })}
      aria-label="Dismiss notice"
      onclick={() => {
        error = '';
        session.notice = '';
      }}>×</button
    >
  </div>{/if}
{#if isHidden}<div class={notice}>
    This topic is inside a collapsed branch.<button
      class={button({ visual: 'quiet' })}
      onclick={reveal}>Reveal in tree</button
    >
  </div>{/if}
<div class={classes.root}>
  <div class={classes.map} inert={!wide.current && exploration.view.mobilePane === 'reader'}>
    <TopicTree
      {exploration}
      onSelect={(id) => navigate(session.select(id))}
      onExpand={(id) => navigate(session.expand(id))}
      onCollapse={(id) => session.collapse(id)}
      onViewportChange={(viewport) => session.setView({ viewport })}
    />
  </div>
  <div class={classes.reader}>
    {#key `${exploration.id}:${selected.id}`}<TopicReader
        node={selected}
        readOnly={session.readOnly ?? false}
        onFollow={follow}
        onRefresh={() => navigate(session.refresh(selected.id))}
        onClarify={(answer) => navigate(session.clarify(selected.id, answer))}
        clarification={session.clarification?.nodeId === selected.id
          ? session.clarification.question
          : null}>{@render readerExtras?.()}</TopicReader
      >{/key}
  </div>
</div>
