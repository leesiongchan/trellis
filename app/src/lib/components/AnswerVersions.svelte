<script lang="ts">
  import { css } from '#styled/css';
  import { button } from '#styled/recipes';
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

<section
  class={css({
    display: 'grid',
    gap: '3',
    pt: '5',
    borderTopWidth: '1px',
    borderColor: 'border',
    textStyle: 'body.sm',
    overflowWrap: 'anywhere'
  })}
  aria-label="Saved answer history"
>
  {#if topic.selectedAnswer && topic.currentAnswerVersionId !== topic.selectedAnswer.id}<p
      class={css({ bg: 'accent.subtle', color: 'accent', p: '3', rounded: 'control' })}
    >
      A newer saved answer is available.
    </p>{/if}
  {#if topic.versions.length}<details
      open={!!topic.selectedAnswer && topic.currentAnswerVersionId !== topic.selectedAnswer.id}
    >
      <summary
        class={css({
          minH: 'control',
          py: '3',
          cursor: 'pointer',
          color: 'accent',
          fontWeight: 'medium'
        })}>Saved answer history · {topic.versions.length} versions</summary
      >
      {#each topic.versions as version (version.id)}<div
          class={css({
            display: 'flex',
            flexWrap: 'wrap',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '3',
            py: '3',
            borderTopWidth: '1px',
            borderColor: 'border'
          })}
        >
          <span>Version {version.revision} · {new Date(version.createdAt).toLocaleString()}</span
          >{#if version.id === topic.selectedAnswer?.id}<span>Selected</span>{:else}<button
              class={button({ visual: 'quiet' })}
              disabled={readOnly}
              onclick={() => onChoose(version.id)}
              >{preview ? 'Read' : 'Use'} version {version.revision}</button
            >{/if}
        </div>{/each}
    </details>{/if}
</section>
