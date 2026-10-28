<script lang="ts">
  import { css, cva } from '#styled/css';
  import type { Source } from '#lib/knowledge/types.ts';
  let {
    source,
    compact = false,
    index = 1
  }: { source: Source; compact?: boolean; index?: number } = $props();
  const citation = cva({
    base: {
      color: 'accent',
      rounded: 'sm',
      textDecoration: 'none',
      _hover: { textDecoration: 'underline', textUnderlineOffset: '3px' }
    },
    variants: {
      compact: {
        true: {
          display: 'inline-block',
          fontSize: 'xs',
          lineHeight: 'tight',
          verticalAlign: 'super',
          ml: '1',
          minW: '4',
          minH: '5',
          textAlign: 'center',
          _focusVisible: { outlineOffset: '0.5' }
        },
        false: { display: 'flex', gap: '3', alignItems: 'start', minH: 'control', py: '3' }
      }
    },
    defaultVariants: { compact: false }
  });
</script>

<a
  class={citation({ compact })}
  href={source.url}
  target="_blank"
  rel="noopener noreferrer"
  title={source.title}
  aria-label={compact ? 'Source ' + index + ': ' + source.title : source.title}
  >{#if compact}{index}{:else}<span
      class={css({ textStyle: 'caption', color: 'fg.muted', flexShrink: '0', minW: '5' })}
      >{index.toString().padStart(2, '0')}</span
    ><span
      class={css({ minW: '0', display: 'grid', gap: '1', flex: '1', overflowWrap: 'anywhere' })}
      ><strong class={css({ textStyle: 'body.sm', fontWeight: 'medium' })}>{source.title}</strong
      ><small class={css({ textStyle: 'caption', color: 'fg.muted' })}
        >{source.publisher ?? new URL(source.url).hostname}</small
      ></span
    ><span class={css({ flexShrink: '0' })} aria-hidden="true">↗</span>{/if}</a
>
