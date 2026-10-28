<script lang="ts">
  import { css } from '#styled/css';
  import { button } from '#styled/recipes';
  import { useSvelteFlow } from '@xyflow/svelte';
  let { onRefocus }: { onRefocus: () => void } = $props();
  const { zoomIn, zoomOut, fitView } = useSvelteFlow();
</script>

<div
  class={css({
    position: 'absolute',
    bottom: '4',
    left: '4',
    display: 'flex',
    alignItems: 'center',
    gap: '1',
    p: '1',
    borderWidth: '1px',
    borderColor: 'border.strong',
    bg: 'bg.surface',
    rounded: 'panel',
    shadow: 'sm',
    zIndex: '10'
  })}
  aria-label="Map controls"
>
  <button class={button({ visual: 'icon' })} aria-label="Zoom out" onclick={() => zoomOut()}
    >−</button
  ><button class={button({ visual: 'icon' })} aria-label="Zoom in" onclick={() => zoomIn()}
    >+</button
  ><span class={css({ h: '5', borderLeftWidth: '1px', borderColor: 'border' })}></span><button
    class={button({ visual: 'icon' })}
    aria-label="Fit entire tree"
    title="Fit entire tree"
    onclick={() => fitView({ padding: 0.2 })}>⛶</button
  ><button
    class={button({ visual: 'icon' })}
    aria-label="Refocus selected topic"
    title="Refocus selected topic"
    onclick={onRefocus}>⊙</button
  >
</div>
