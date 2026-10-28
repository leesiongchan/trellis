<script lang="ts">
  import { css } from '#styled/css';
  import { button } from '#styled/recipes';
  import type { Page, ConnectionView } from '#lib/knowledge/library-types.ts';
  import CitationLink from './CitationLink.svelte';
  let {
    connections,
    topicId,
    loading = false,
    discovery = 'unavailable',
    offline = false,
    onFollow,
    onDismiss,
    onMore,
    onRefresh
  }: {
    connections: Page<ConnectionView>;
    topicId: string;
    loading?: boolean;
    discovery?: string;
    offline?: boolean;
    onFollow: (id: string) => void;
    onDismiss: (id: string) => void;
    onMore: () => void;
    onRefresh: () => void;
  } = $props();
  const labels = {
    branches_to: 'Exploration branch',
    equivalent_question: 'Equivalent question',
    similar_question: 'Similar question',
    overlapping_answer: 'Overlapping answer',
    related_topic: 'Related topic'
  };
</script>

<section
  class={css({
    display: 'grid',
    gap: '4',
    pt: '5',
    borderTopWidth: '1px',
    borderColor: 'border',
    textStyle: 'body.sm',
    minW: '0',
    overflowWrap: 'anywhere'
  })}
  aria-label="Topic connections"
  aria-busy={loading}
>
  <div
    class={css({
      display: 'flex',
      flexWrap: 'wrap',
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: '2',
      textStyle: 'label',
      color: 'fg.muted'
    })}
  >
    SAVED CONNECTIONS <button
      class={button({ visual: 'quiet', size: 'sm' })}
      onclick={onRefresh}
      disabled={offline || loading}>Refresh connections</button
    >
  </div>
  {#if offline}<p>
      Connections require a library connection. Your saved explanation stays readable.
    </p>{:else if discovery === 'unavailable'}<p>
      Related-topic discovery is unavailable. Previously saved connections remain available.
    </p>{:else if discovery === 'indexing'}<p role="status">
      Finding possible connections in saved topics…
    </p>{/if}
  {#if !connections.items.length && !loading && !offline && discovery === 'ready'}<p>
      No saved connections yet.
    </p>{/if}
  {#each connections.items as link (link.id)}
    <article
      class={css({
        display: 'grid',
        gap: '3',
        p: '4',
        bg: 'bg.surface',
        borderWidth: '1px',
        borderColor: 'border.strong',
        rounded: 'panel',
        minW: '0'
      })}
      aria-label={labels[link.kind] + ' connection'}
      data-connection-id={link.id}
    >
      <div
        class={css({
          display: 'flex',
          flexWrap: 'wrap',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '2',
          textStyle: 'caption',
          color: 'fg.muted'
        })}
      >
        {labels[link.kind]}
        <span>{link.status === 'candidate' ? 'Possible connection' : 'Saved link'}</span>
      </div>
      <strong>{link.fromTopicId === topicId ? link.toQuestion : link.fromQuestion}</strong>
      <p>{link.reason}</p>
      {#if link.citationSources.length}<details>
          <summary class={css({ minH: 'control', py: '3', cursor: 'pointer', color: 'accent' })}
            >Saved passage evidence</summary
          >{#each link.citationSources as citation, index (index)}<blockquote
              class={css({
                p: '3',
                mb: '3',
                bg: 'bg.subtle',
                borderLeftWidth: '2px',
                borderColor: 'accent',
                overflowWrap: 'anywhere'
              })}
            >
              {citation.quote}<CitationLink source={citation.source} compact index={index + 1} />
            </blockquote>{/each}
        </details>{/if}
      <div class={css({ display: 'flex', flexWrap: 'wrap', gap: '2' })}>
        <button
          class={button({ visual: 'quiet' })}
          disabled={offline}
          onclick={() => onFollow(link.id)}>Open saved topic</button
        >{#if link.kind !== 'branches_to'}<button
            class={button({ visual: 'quiet' })}
            disabled={offline}
            aria-label="Dismiss connection"
            onclick={() => onDismiss(link.id)}>Dismiss</button
          >{/if}
      </div>
    </article>
  {/each}
  {#if connections.nextCursor}<button
      class={button({ visual: 'quiet' })}
      disabled={loading || offline}
      onclick={onMore}>More connections</button
    >{/if}
</section>
