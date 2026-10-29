<script module lang="ts">
  import type { Node } from '@xyflow/svelte';
  import type { TopicNode as Topic } from '#lib/knowledge/types.ts';
  export type FlowTopicNode = Node<
    {
      topic: Topic;
      expanded: boolean;
      active: boolean;
      onSelect: (id: string) => void;
      onExpand: (id: string) => void;
      onCollapse: (id: string) => void;
    },
    'topic'
  >;
</script>

<script lang="ts">
  import { css, sva } from '#styled/css';
  import { Handle, Position, type NodeProps } from '@xyflow/svelte';
  import BotanicalMark from './BotanicalMark.svelte';
  let { data }: NodeProps<FlowTopicNode> = $props();
  const topic = $derived(data.topic);
  const node = sva({
    slots: ['root', 'read', 'relationship', 'title', 'status', 'expand', 'count'],
    base: {
      // SvelteFlow layout and node bounds use fixed 240 × 150 CSS-pixel geometry.
      root: {
        w: '[240px]',
        h: '[150px]',
        display: 'flex',
        flexDirection: 'column',
        bg: 'bg.surface',
        borderWidth: '1px',
        borderColor: 'border.strong',
        rounded: 'panel',
        shadow: 'xs',
        transitionProperty: 'border-color, box-shadow',
        transitionDuration: 'fast',
        animationName: 'branchEnter',
        animationDuration: 'normal',
        animationTimingFunction: 'standard',
        _motionReduce: { transitionProperty: 'none', animationName: 'none' },
        overflow: 'hidden',
        _focusWithin: { borderColor: 'focus' }
      },
      read: {
        flex: '1',
        minH: '0',
        w: 'full',
        display: 'flex',
        flexDirection: 'column',
        gap: '1',
        justifyContent: 'center',
        px: '4',
        py: '2',
        textAlign: 'left',
        bg: 'transparent',
        _focusVisible: { outlineOffset: '-1' }
      },
      relationship: {
        display: 'flex',
        alignItems: 'center',
        gap: '1.5',
        fontSize: 'sm',
        color: 'fg.muted',
        fontWeight: 'semibold',
        lineHeight: 'tight',
        overflow: 'hidden',
        whiteSpace: 'nowrap',
        textOverflow: 'ellipsis'
      },
      title: {
        fontSize: 'lg',
        lineHeight: 'tight',
        fontWeight: 'semibold',
        lineClamp: 2,
        overflowWrap: 'anywhere'
      },
      status: { fontSize: 'sm', lineHeight: 'tight', color: 'fg.muted' },
      expand: {
        display: 'flex',
        alignItems: 'center',
        gap: '2',
        minH: 'control',
        w: 'full',
        px: '4',
        fontSize: 'sm',
        textAlign: 'left',
        bg: 'bg.surface',
        color: 'accent',
        borderTopWidth: '1px',
        borderColor: 'border',
        flexShrink: '0',
        _hover: { bg: 'accent.subtle' },
        _focusVisible: { outlineOffset: '-1' }
      },
      count: { ml: 'auto' }
    },
    variants: {
      root: { true: { root: { bg: 'bg.highlight', borderBottomLeftRadius: 'sm' } } },
      active: { true: { root: { borderColor: 'accent', borderWidth: '2px', shadow: 'sm' } } },
      status: {
        suggested: {},
        ready: {},
        researching: { root: { borderStyle: 'dashed' } },
        failed: { root: { borderColor: 'danger' }, status: { color: 'danger' } }
      }
    }
  });
  const classes = $derived(
    node({ active: data.active, root: topic.parentId === null, status: topic.status })
  );
</script>

<div class={classes.root}>
  {#if topic.parentId}<Handle type="target" position={Position.Left} isConnectable={false} />{/if}
  <button
    class={[classes.read, 'nodrag', 'nopan']}
    aria-label={'Read ' + topic.question}
    aria-describedby={topic.status === 'failed' ? 'topic-status-' + topic.id : undefined}
    aria-current={data.active ? 'location' : undefined}
    onclick={() => data.onSelect(topic.id)}
    >{#if topic.status === 'failed' || topic.status === 'researching'}
      <span class={classes.status} id={'topic-status-' + topic.id} data-topic-label>
        {topic.status === 'failed' ? 'Research failed. Open to retry.' : 'Researching…'}
      </span>
    {:else}
      <span class={classes.relationship} data-topic-label
        ><BotanicalMark /><span
          class={css({ minW: '0', overflow: 'hidden', textOverflow: 'ellipsis' })}
          >{topic.relationship || 'Your starting question'}</span
        ></span
      >
    {/if}
    <strong class={classes.title}>{topic.content?.title ?? topic.question}</strong></button
  >
  <button
    class={[classes.expand, 'nodrag', 'nopan']}
    aria-label={(data.expanded ? 'Collapse ' : 'Expand ') + topic.question}
    aria-expanded={data.expanded}
    onclick={() => (data.expanded ? data.onCollapse(topic.id) : data.onExpand(topic.id))}
    ><span>{data.expanded ? '−' : topic.status === 'researching' ? '◌' : '+'}</span>{data.expanded
      ? 'Collapse branches'
      : topic.status === 'researching'
        ? 'Researching…'
        : topic.childIds.length
          ? 'Explore ' + topic.childIds.length + ' branches'
          : 'Explore branches'}<span class={classes.count}
      >{topic.status === 'ready' ? '↗' : '·'}</span
    ></button
  >
  <Handle type="source" position={Position.Right} isConnectable={false} />
</div>
