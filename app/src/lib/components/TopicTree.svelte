<script lang="ts">
  import { SvelteFlowProvider } from '@xyflow/svelte';
  import type { ComponentProps } from 'svelte';
  import TreeCanvas from './TreeCanvas.svelte';
  let props: ComponentProps<typeof TreeCanvas> = $props();
</script>

{#key props.exploration.id}
  <SvelteFlowProvider><TreeCanvas {...props} /></SvelteFlowProvider>
{/key}
