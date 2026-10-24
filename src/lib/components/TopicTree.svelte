<script lang="ts">
  import { SvelteFlowProvider } from '@xyflow/svelte';
  import type { ComponentProps } from 'svelte';
  import TreeCanvas from './TreeCanvas.svelte';
  let props:ComponentProps<typeof TreeCanvas>=$props();
</script>
<SvelteFlowProvider><TreeCanvas {...props}/></SvelteFlowProvider>
