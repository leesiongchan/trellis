<script lang="ts">
  import { css } from '#styled/css';
  import { button } from '#styled/recipes';
  import type { LibraryInfo } from '#lib/knowledge/library-types.ts';
  import { PURGE_CONFIRMATION } from '#lib/knowledge/library-schemas.ts';
  let {
    library,
    offline,
    busy = false,
    onPurge
  }: {
    library: LibraryInfo | null;
    offline: boolean;
    busy?: boolean;
    onPurge: (confirmation: string) => void;
  } = $props();
  let confirming = $state(false),
    confirmation = $state('');
</script>

<section
  class={css({
    display: 'grid',
    gap: '3',
    textStyle: 'body.sm',
    color: 'fg.muted',
    overflowWrap: 'anywhere',
    minW: '0'
  })}
  aria-label="Library storage"
>
  <p>
    {offline
      ? 'The server is unreachable. Reconnect to load or change saved content.'
      : 'Explorations and saved knowledge are shared by holders of your private access link.'}
  </p>
  {#if library?.discovery === 'unavailable'}<p>
      Related-topic discovery is unavailable; your saved library is available.
    </p>{:else if library?.discovery === 'indexing'}<p role="status">
      Finding connections in saved knowledge…
    </p>{/if}
  <div class={css({ display: 'flex', flexWrap: 'wrap', gap: '2' })}>
    <button
      class={button({ visual: 'quiet' })}
      disabled={offline || busy || !library}
      onclick={() => (confirming = !confirming)}>Clear saved library</button
    >
  </div>
  {#if confirming}<form
      class={css({
        display: 'grid',
        gap: '3',
        p: '4',
        bg: 'danger.subtle',
        borderWidth: '1px',
        borderColor: 'danger',
        rounded: 'panel',
        color: 'fg',
        minW: '0'
      })}
      onsubmit={(event) => {
        event.preventDefault();
        if (confirmation === PURGE_CONFIRMATION) {
          onPurge(confirmation);
          confirmation = '';
          confirming = false;
        }
      }}
    >
      <p>
        Delete all server questions, answers, explorations, connections, and derived indexes?
        Retained backups are managed separately.
      </p>
      <p>Type this confirmation:</p>
      <p class={css({ fontFamily: 'mono', fontSize: 'sm', overflowWrap: 'anywhere' })}>
        {PURGE_CONFIRMATION}
      </p>
      <label for="library-confirmation">Library deletion confirmation</label><textarea
        id="library-confirmation"
        class={css({
          w: 'full',
          minW: '0',
          minH: '24',
          bg: 'bg.surface',
          borderWidth: '1px',
          borderColor: 'border.strong',
          rounded: 'control',
          px: '3',
          py: '2',
          fontSize: 'md',
          resize: 'vertical'
        })}
        bind:value={confirmation}
        required></textarea><button
        class={button({ visual: 'danger' })}
        disabled={busy || offline || confirmation !== PURGE_CONFIRMATION}
        >Delete all server library content</button
      ><button
        type="button"
        class={button({ visual: 'quiet' })}
        onclick={() => (confirming = false)}>Keep saved library</button
      >
    </form>{/if}
</section>
