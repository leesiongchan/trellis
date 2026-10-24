<script lang="ts">
  import { useSvelteFlow } from '@xyflow/svelte';
  let {onRefocus}:{onRefocus:()=>void}=$props();const {zoomIn,zoomOut,fitView}=useSvelteFlow();
</script>
<div class="map-controls" aria-label="Map controls"><button aria-label="Zoom out" onclick={()=>zoomOut()}>−</button><button aria-label="Zoom in" onclick={()=>zoomIn()}>+</button><span></span><button aria-label="Fit entire tree" title="Fit entire tree" onclick={()=>fitView({padding:.2})}>⛶</button><button aria-label="Refocus selected topic" title="Refocus selected topic" onclick={onRefocus}>⊙</button></div>
