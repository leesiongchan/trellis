<script lang="ts">
  import { css } from '#styled/css';
  import { button } from '#styled/recipes';
  import BotanicalMark from './BotanicalMark.svelte';
  import type { SavedExploration } from '#lib/knowledge/types.ts';
  let {
    explorations,
    onResume,
    onDelete
  }: {
    explorations: SavedExploration[];
    onResume: (id: string) => void;
    onDelete: (id: string) => void;
  } = $props();
  let deleting = $state<string | null>(null);
</script>

{#if explorations.length}
  <section
    class={css({ display: 'grid', gap: '3', minW: '0', containerType: 'inline-size' })}
    aria-label="Saved explorations"
  >
    <div
      class={css({
        textStyle: 'label',
        display: 'flex',
        flexWrap: 'wrap',
        gap: '3',
        justifyContent: 'space-between',
        color: 'fg.muted',
        mb: '2'
      })}
    >
      PICK UP A THREAD <span>{explorations.length} saved</span>
    </div>
    {#each explorations as item (item.id)}
      <div
        class={css({
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr)',
          '@container (min-width: 26rem)': { gridTemplateColumns: 'minmax(0, 1fr) auto' },
          gap: '2',
          alignItems: 'start',
          borderWidth: '1px',
          borderColor: 'border',
          p: '3',
          rounded: 'panel',
          bg: 'bg.surface'
        })}
      >
        <button
          class={css({
            display: 'flex',
            gap: '3',
            alignItems: 'start',
            textAlign: 'left',
            minW: '0',
            minH: 'control',
            p: '2',
            rounded: 'control',
            bg: 'transparent',
            _hover: { bg: 'accent.subtle' }
          })}
          aria-label={'Resume ' + item.originalPrompt}
          onclick={() => onResume(item.id)}
        >
          <span
            class={css({
              p: '2',
              bg: 'bg.highlight',
              rounded: 'full',
              flexShrink: '0',
              display: 'none',
              '@container (min-width: 26rem)': { display: 'block' }
            })}><BotanicalMark /></span
          ><span class={css({ minW: '0', display: 'grid', gap: '2' })}
            ><strong
              class={css({ textStyle: 'body.sm', fontWeight: 'medium', overflowWrap: 'anywhere' })}
              >{item.originalPrompt}</strong
            ><small
              class={css({ textStyle: 'caption', color: 'fg.muted', overflowWrap: 'anywhere' })}
              >{#if item.importedFrom}<span
                  >{item.importedFrom.changedCopy
                    ? 'Changed browser copy'
                    : 'Imported browser copy'}</span
                > ·
              {/if}{item.topicCount !== undefined ? item.topicCount + ' topics · ' : ''}
              {new Date(item.updatedAt).toLocaleDateString(undefined, {
                month: 'short',
                day: 'numeric'
              })}</small
            ></span
          >
        </button>
        <button
          class={[button({ visual: 'icon' }), css({ justifySelf: 'end' })]}
          aria-label={'Delete ' + item.originalPrompt}
          onclick={() => (deleting = item.id)}>×</button
        >
        {#if deleting === item.id}<div
            class={css({
              gridColumn: '1 / -1',
              display: 'flex',
              flexWrap: 'wrap',
              gap: '2',
              p: '3',
              bg: 'danger.subtle',
              rounded: 'control',
              textStyle: 'body.sm',
              overflowWrap: 'anywhere'
            })}
            role="alert"
          >
            <p>Delete this exploration? Saved topics and answer history remain in the library.</p>
            <button class={button({ visual: 'quiet' })} onclick={() => (deleting = null)}
              >Keep it</button
            ><button
              class={button({ visual: 'danger' })}
              onclick={() => {
                onDelete(item.id);
                deleting = null;
              }}>Delete exploration</button
            >
          </div>{/if}
      </div>
    {/each}
  </section>
{/if}
