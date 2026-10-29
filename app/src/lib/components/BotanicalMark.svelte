<script lang="ts">
  import { css } from '#styled/css';
  let { kind = 'leaf' }: { kind?: 'leaf' | 'spark' } = $props();
</script>

<svg
  class={css({ w: '5', h: '5', flexShrink: '0', color: 'ornament' })}
  viewBox="0 0 24 24"
  fill="none"
  aria-hidden="true"
>
  {#if kind === 'leaf'}
    <path d="M3 20C3 8 11 2 22 3C22 15 14 22 3 20Z" fill="currentColor" />
    <path d="M5 18L17 7" stroke="currentColor" stroke-width="1.5" />
  {:else}
    <path
      d="M12 2V22M2 12H22M5 5L19 19M5 19L19 5"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
    />
  {/if}
</svg>
