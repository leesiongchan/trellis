<script lang="ts">
  import type { TopicView } from '#lib/knowledge/library-types.ts';
  let {
    topic,
    onChoose,
    readOnly = false,
    preview = false
  }: {
    topic: TopicView;
    onChoose: (id: string) => void;
    readOnly?: boolean;
    preview?: boolean;
  } = $props();
</script>

<section class="answer-versions" aria-label="Saved answer history">
  {#if topic.selectedAnswer && topic.currentAnswerVersionId !== topic.selectedAnswer.id}<p
      class="newer-answer"
    >
      A newer saved answer is available.
    </p>{/if}
  {#if topic.versions.length}<details
      open={!!topic.selectedAnswer && topic.currentAnswerVersionId !== topic.selectedAnswer.id}
    >
      <summary>Saved answer history · {topic.versions.length} versions</summary>
      {#each topic.versions as version (version.id)}<div class="answer-version">
          <span>Version {version.revision} · {new Date(version.createdAt).toLocaleString()}</span
          >{#if version.id === topic.selectedAnswer?.id}<span>Selected</span>{:else}<button
              class="quiet-button"
              disabled={readOnly}
              onclick={() => onChoose(version.id)}
              >{preview ? 'Read' : 'Use'} version {version.revision}</button
            >{/if}
        </div>{/each}
    </details>{/if}
</section>
