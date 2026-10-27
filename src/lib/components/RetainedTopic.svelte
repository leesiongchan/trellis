<script lang="ts">
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

<main class="retained-topic">
  <div class="retained-heading">
    <p>Read-only saved topic</p>
    <button class="primary-button" {disabled} onclick={() => onStartExploration(topic.id)}
      >Start exploration from this topic</button
    >
    <p>
      Starts a separate exploration with this saved context and answer. Existing appearances keep
      their selected answers.
    </p>
  </div>
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
</main>
