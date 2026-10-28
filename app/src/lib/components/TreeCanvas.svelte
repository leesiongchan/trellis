<script lang="ts">
  import { SvelteFlow, Background, useSvelteFlow, type Viewport } from '@xyflow/svelte';
  import { tick, untrack } from 'svelte';
  import type { WorkspaceExploration } from '#lib/knowledge/types.ts';
  import { layoutTree, visibleTree } from '#lib/knowledge/tree.ts';
  import TopicNode, { type FlowTopicNode } from './TopicNode.svelte';
  import WorkspaceControls from './WorkspaceControls.svelte';
  let {
    exploration,
    onSelect,
    onExpand,
    onCollapse,
    onViewportChange
  }: {
    exploration: WorkspaceExploration;
    onSelect: (id: string) => void;
    onExpand: (id: string) => void;
    onCollapse: (id: string) => void;
    onViewportChange: (viewport: Viewport) => void;
  } = $props();
  const flow = useSvelteFlow<FlowTopicNode>(),
    nodeTypes = { topic: TopicNode };
  const visible = $derived(visibleTree(exploration));
  const nodes = $derived.by(() => {
    const positions = new Map(layoutTree(exploration).map((p) => [p.id, p]));
    return visible.nodes.map((topic) => ({
      id: topic.id,
      type: 'topic' as const,
      position: { x: positions.get(topic.id)!.x, y: positions.get(topic.id)!.y },
      width: 240,
      height: 150,
      selected: topic.id === exploration.view.selectedNodeId,
      data: {
        topic,
        expanded: exploration.view.expandedNodeIds.includes(topic.id),
        active: topic.id === exploration.view.selectedNodeId,
        onSelect,
        onExpand,
        onCollapse
      }
    }));
  });
  const edges = $derived(
    visible.edges.map((e) => ({
      ...e,
      type: 'smoothstep',
      reconnectable: false,
      style: 'stroke: #a7baa0; stroke-width: 1.3;'
    }))
  );
  const initialViewport = untrack(() => exploration.view.viewport);
  let previousSignature = '',
    mounted = false;
  function refocus() {
    const node = exploration.nodes[exploration.view.selectedNodeId];
    void flow.fitView({
      nodes: [
        { id: node.id },
        ...node.childIds
          .filter((id) => visible.nodes.some((n) => n.id === id))
          .map((id) => ({ id }))
      ],
      padding: 0.3,
      minZoom: 0.5,
      maxZoom: 1,
      duration: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 200
    });
  }
  $effect(() => {
    const signature =
      exploration.id +
      ':' +
      exploration.view.selectedNodeId +
      ':' +
      nodes.map((n) => n.id).join(',');
    if (signature !== previousSignature) {
      const shouldFocus = mounted || !exploration.viewRestored;
      previousSignature = signature;
      mounted = true;
      if (shouldFocus) void tick().then(refocus);
    }
  });
</script>

<div class="topic-map" role="region" aria-label="Topic map">
  <SvelteFlow
    {nodes}
    {edges}
    {nodeTypes}
    {initialViewport}
    nodesDraggable={false}
    nodesConnectable={false}
    deleteKey={null}
    minZoom={0.1}
    maxZoom={2}
    panOnScroll
    zoomOnScroll={false}
    onmoveend={(_event, viewport) => onViewportChange(viewport)}
  >
    <Background patternColor="#ced7c8" gap={22} size={1} />
  </SvelteFlow>
  <WorkspaceControls onRefocus={refocus} />
  <div class="canvas-caption">
    <span class="green-dot"></span>
    {visible.nodes.length} topics in view <span class="caption-divider">/</span> Follow the + to go deeper
  </div>
</div>
