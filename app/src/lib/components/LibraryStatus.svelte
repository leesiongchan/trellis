<script lang="ts">
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

<section class="library-status" aria-label="Library storage">
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
  <div class="connection-actions">
    <button
      class="quiet-button"
      disabled={offline || busy || !library}
      onclick={() => (confirming = !confirming)}>Clear saved library</button
    >
  </div>
  {#if confirming}<form
      class="purge-confirmation"
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
      <p class="confirmation-phrase">{PURGE_CONFIRMATION}</p>
      <label for="library-confirmation">Library deletion confirmation</label><textarea
        id="library-confirmation"
        bind:value={confirmation}
        required></textarea><button
        class="danger-button"
        disabled={busy || offline || confirmation !== PURGE_CONFIRMATION}
        >Delete all server library content</button
      ><button type="button" class="quiet-button" onclick={() => (confirming = false)}
        >Keep saved library</button
      >
    </form>{/if}
</section>
