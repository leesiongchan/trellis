<script lang="ts">
  import { css } from '#styled/css';
  import { button } from '#styled/recipes';
  import type { TopicView } from '#lib/knowledge/library-types.ts';
  import type { TopicNode } from '#lib/knowledge/types.ts';
  import type { Snippet } from 'svelte';
  import TopicReader from './TopicReader.svelte';
  import AnswerVersions from './AnswerVersions.svelte';
  let {
    topic,
    disabled = false,
    onStartExploration,
    onReadVersion,
    children
  }: {
    topic: TopicView;
    disabled?: boolean;
    onStartExploration: (id: string) => void;
    onReadVersion: (id: string) => void;
    children?: Snippet;
  } = $props();
  const node = $derived<TopicNode>({
    id: topic.id,
    parentId: null,
    question: topic.question,
    relationship: 'Saved topic',
    teaser: '',
    status: topic.selectedAnswer ? 'ready' : 'suggested',
    content: topic.selectedAnswer?.content ?? null,
    childIds: [],
    lastError: null,
    isRefreshing: false
  });
</script>

<main
  class={css({
    flex: '1',
    minH: '0',
    minW: '0',
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
    bg: 'bg.surface'
  })}
>
  <div
    class={css({
      display: 'grid',
      gap: '3',
      px: 'section',
      py: '5',
      bg: 'accent.subtle',
      borderBottomWidth: '1px',
      borderColor: 'border',
      textStyle: 'body.sm',
      overflowWrap: 'anywhere'
    })}
  >
    <p class={css({ textStyle: 'label', color: 'accent' })}>Read-only saved topic</p>
    <button
      class={button({ visual: 'primary' })}
      {disabled}
      onclick={() => onStartExploration(topic.id)}>Start exploration from this topic</button
    >
    <p>
      Starts a separate exploration with this saved context and answer. Existing appearances keep
      their selected answers.
    </p>
  </div>
  <div class={css({ flex: '1', minH: '0' })}>
    <TopicReader
      {node}
      readOnly
      onFollow={() => {}}
      onRefresh={() => {}}
      onClarify={() => {}}
      clarification={null}
    >
      <AnswerVersions {topic} preview onChoose={onReadVersion} readOnly={disabled} />
      {@render children?.()}
    </TopicReader>
  </div>
</main>
