<script lang="ts">
  import { css, sva } from '#styled/css';
  import { button } from '#styled/recipes';

  let {
    exploring,
    saveState,
    disabled,
    explorationCount,
    onHome,
    onSavedTopics,
    onExplorations
  }: {
    exploring: boolean;
    saveState: 'saved' | 'saving' | 'failed' | 'conflict' | 'offline';
    disabled: boolean;
    explorationCount: number;
    onHome: () => void;
    onSavedTopics: () => void;
    onExplorations: () => void;
  } = $props();
  const header = sva({
    slots: ['root', 'wordmark', 'tagline'],
    base: {
      root: {
        display: 'flex',
        flexWrap: 'wrap',
        alignItems: 'center',
        columnGap: '6',
        rowGap: '2',
        px: 'gutter',
        py: '4',
        flexShrink: '0',
        borderBottomWidth: '1px',
        borderColor: 'border',
        bg: 'bg.surface'
      },
      wordmark: {
        fontFamily: 'heading',
        fontSize: '4xl',
        letterSpacing: 'tighter',
        lineHeight: 'none',
        minH: 'control',
        bg: 'transparent',
        display: 'flex',
        alignItems: 'center'
      },
      tagline: {
        display: 'none',
        lg: { display: 'block' },
        textStyle: 'caption',
        color: 'fg.muted',
        borderLeftWidth: '1px',
        borderColor: 'border',
        pl: '6'
      }
    },
    variants: {
      exploring: {
        true: {
          root: { py: '2' },
          wordmark: { fontSize: '3xl' },
          tagline: { display: 'none', lg: { display: 'none' } }
        }
      }
    }
  });
  const classes = $derived(header({ exploring }));
  const labels = {
    saved: 'All changes saved',
    saving: 'Saving…',
    failed: 'Not saved',
    conflict: 'Changed in another tab',
    offline: 'Server disconnected'
  };
</script>

<header class={classes.root}>
  <button class={classes.wordmark} aria-label="Trellis home" onclick={onHome}
    >trellis<span class={css({ color: 'ornament' })}>.</span></button
  >
  <span class={classes.tagline}>Room for a little wonder.</span>
  <div
    class={css({
      display: 'flex',
      flexWrap: 'wrap',
      alignItems: 'center',
      gap: '1',
      ml: 'auto',
      maxW: 'full'
    })}
  >
    {#if exploring}
      <span
        role="status"
        class={css({
          textStyle: 'caption',
          color: saveState === 'failed' || saveState === 'conflict' ? 'danger' : 'fg.muted',
          px: '2'
        })}>{labels[saveState]}</span
      >
      <button class={button({ visual: 'quiet' })} onclick={onHome}>+ New exploration</button>
    {/if}
    <button class={button({ visual: 'quiet' })} {disabled} onclick={onSavedTopics}
      >Saved topics</button
    >
    <button
      class={button({ visual: 'outline' })}
      aria-label="Your explorations"
      {disabled}
      onclick={onExplorations}
    >
      Your explorations{#if explorationCount}<span
          class={css({
            textStyle: 'caption',
            bg: 'accent.subtle',
            color: 'accent',
            rounded: 'full',
            minW: '5',
            px: '1'
          })}>{explorationCount}</span
        >{/if}
    </button>
  </div>
</header>
