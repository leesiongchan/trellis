<script lang="ts">
  import type { Source } from '#lib/knowledge/types.ts';
  let {
    source,
    compact = false,
    index = 1
  }: { source: Source; compact?: boolean; index?: number } = $props();
</script>

<a
  class:compact-citation={compact}
  class:source-link={!compact}
  href={source.url}
  target="_blank"
  rel="noopener noreferrer"
  title={source.title}
  aria-label={compact ? 'Source ' + index + ': ' + source.title : source.title}
  >{#if compact}<sup>{index}</sup>{:else}<span class="source-number"
      >{index.toString().padStart(2, '0')}</span
    ><span
      ><strong>{source.title}</strong><small
        >{source.publisher ?? new URL(source.url).hostname}</small
      ></span
    ><span class="external-arrow" aria-hidden="true">↗</span>{/if}</a
>
