<script lang="ts">
  import type { Exploration } from '#lib/knowledge/types';
  let { explorations, onResume, onDelete }: { explorations:Exploration[];onResume:(id:string)=>void;onDelete:(id:string)=>void }=$props();
  let deleting=$state<string|null>(null);
</script>
{#if explorations.length}
  <section class="saved-explorations" aria-label="Saved explorations">
    <div class="section-label">PICK UP A THREAD <span>{explorations.length} saved</span></div>
    {#each explorations as item(item.id)}
      <div class="saved-item">
        <button class="saved-resume" aria-label={'Resume '+item.originalPrompt} onclick={()=>onResume(item.id)}>
          <span class="saved-icon">↗</span><span><strong>{item.originalPrompt}</strong><small>{Object.keys(item.nodes).length} topics · {new Date(item.updatedAt).toLocaleDateString(undefined,{month:'short',day:'numeric'})}</small></span>
        </button>
        <button class="icon-button" aria-label={'Delete '+item.originalPrompt} onclick={()=>deleting=item.id}>×</button>
        {#if deleting===item.id}<div class="delete-confirm" role="alert"><p>Delete this exploration from this browser?</p><button class="quiet-button" onclick={()=>deleting=null}>Keep it</button><button class="danger-button" onclick={()=>{onDelete(item.id);deleting=null;}}>Delete exploration</button></div>{/if}
      </div>
    {/each}
  </section>
{/if}
