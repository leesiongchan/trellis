<script lang="ts">
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

<div class="workspace-toolbar">
  <nav aria-label="Topic path">
    <button
      class="root-button"
      aria-label="Return to root"
      onclick={() => navigate(session.select(exploration.rootId))}>⌂</button
    >{#each path as node, i (node.id)}{#if i}<span class="path-arrow">›</span>{/if}<button
        class:current={node.id === selected.id}
        onclick={() => navigate(session.select(node.id))}
        >{node.content?.title ?? node.question}</button
      >{/each}
  </nav>
  <div class="pane-toggle">
    <button
      class:chosen={exploration.view.mobilePane === 'map'}
      onclick={() => session.setView({ mobilePane: 'map' })}>Map</button
    ><button
      class:chosen={exploration.view.mobilePane === 'reader'}
      onclick={() => session.setView({ mobilePane: 'reader' })}>Read</button
    >
  </div>
  <button class="quiet-button collapse-all" onclick={() => session.setView({ expandedNodeIds: [] })}
    >Collapse branches</button
  >
</div>
{#if error || session.notice}<div class="workspace-notice" role="status">
    {error || session.notice}<button
      class="icon-button"
      aria-label="Dismiss notice"
      onclick={() => {
        error = '';
        session.notice = '';
      }}>×</button
    >
  </div>{/if}
{#if isHidden}<div class="workspace-notice">
    This topic is inside a collapsed branch.<button class="quiet-button" onclick={reveal}
      >Reveal in tree</button
    >
  </div>{/if}
<div
  class="workspace"
  class:show-map={exploration.view.mobilePane === 'map'}
  class:show-reader={exploration.view.mobilePane === 'reader'}
>
  <div class="map-pane">
    <TopicTree
      {exploration}
      onSelect={(id) => navigate(session.select(id))}
      onExpand={(id) => navigate(session.expand(id))}
      onCollapse={(id) => session.collapse(id)}
      onViewportChange={(viewport) => session.setView({ viewport })}
    />
  </div>
  {#key `${exploration.id}:${selected.id}`}<TopicReader
      node={selected}
      readOnly={session.readOnly ?? false}
      demo={exploration.mode === 'demo'}
      onFollow={follow}
      onRefresh={() => navigate(session.refresh(selected.id))}
      onClarify={(answer) => navigate(session.clarify(selected.id, answer))}
      clarification={session.clarification?.nodeId === selected.id
        ? session.clarification.question
        : null}>{@render readerExtras?.()}</TopicReader
    >{/key}
</div>
