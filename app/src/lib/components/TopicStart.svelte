<script lang="ts">
  import { css } from '#styled/css';
  let { onStart, disabled = false }: { onStart: (question: string) => void; disabled?: boolean } =
    $props();
  let question = $state('');
  let input: HTMLInputElement;
  const examples = [
    'microSD cards',
    'Why do we dream?',
    'The origins of language',
    'How cities grow'
  ];
  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (question.trim()) onStart(question.trim());
  }
</script>

<section class={css({ containerType: 'inline-size', py: 'section', minW: '0' })}>
  <div
    class={css({
      textStyle: 'label',
      color: 'accent',
      display: 'flex',
      alignItems: 'center',
      gap: '2'
    })}
  >
    <span class={css({ fontSize: 'xl' })} aria-hidden="true">↗</span> FOLLOW A QUESTION SOMEWHERE NEW
  </div>
  <h1 class={css({ textStyle: 'heading.hero', my: '6', textWrap: 'balance' })}>
    A little curiosity.<br />An <em class={css({ color: 'accent' })}>entire world.</em>
  </h1>
  <p class={css({ textStyle: 'body', color: 'fg.muted', maxW: 'prose' })}>
    Start with something you wonder about. Understand it deeply. Then follow the branches to
    whatever catches your eye.
  </p>
  <form class={css({ display: 'grid', gap: '3', mt: '8' })} onsubmit={submit}>
    <label for="topic-input" class={css({ textStyle: 'body.sm', fontWeight: 'medium' })}
      >What would you like to understand?</label
    >
    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: 'auto minmax(0, 1fr)',
        gap: '2',
        alignItems: 'center',
        p: '2',
        pl: '3',
        borderWidth: '1px',
        borderColor: 'border.strong',
        rounded: 'panel',
        bg: 'bg.surface',
        _focusWithin: {
          outlineWidth: '2px',
          outlineStyle: 'solid',
          outlineColor: 'focus',
          outlineOffset: '1'
        },
        '@container (min-width: 26rem)': { gridTemplateColumns: 'auto minmax(0, 1fr) auto' }
      })}
    >
      <span class={css({ color: 'accent', fontSize: '2xl' })} aria-hidden="true">✳</span><input
        class={css({
          minW: '0',
          w: 'full',
          minH: 'control',
          bg: 'transparent',
          border: 'none',
          color: 'fg',
          fontSize: 'md',
          _placeholder: { color: 'fg.muted' },
          _focusVisible: { outline: 'none' }
        })}
        id="topic-input"
        bind:this={input}
        bind:value={question}
        maxlength="500"
        placeholder="How can a tiny card hold so much?"
        autocomplete="off"
        required
        {disabled}
      /><button
        class={css({
          gridColumn: '1 / -1',
          minH: 'control',
          px: '5',
          py: '3',
          rounded: 'control',
          bg: 'accent',
          color: 'bg.surface',
          fontSize: 'sm',
          fontWeight: 'medium',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          gap: '3',
          _hover: { bg: 'accent.hover' },
          '@container (min-width: 26rem)': { gridColumn: 'auto' }
        })}
        aria-label="Start exploring"
        type="submit"
        disabled={disabled || !question.trim()}>Explore <span aria-hidden="true">↗</span></button
      >
    </div>
  </form>
  <div class={css({ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: '2', mt: '5' })}>
    <span class={css({ textStyle: 'caption', color: 'fg.muted', w: 'full', mb: '1' })}
      >A few starting points</span
    >{#each examples as example}<button
        class={css({
          textStyle: 'caption',
          minH: 'control',
          px: '3',
          rounded: 'full',
          borderWidth: '1px',
          borderColor: 'border',
          color: 'accent',
          bg: 'bg.surface',
          _hover: { bg: 'accent.subtle', borderColor: 'accent' }
        })}
        onclick={() => {
          question = example;
          input.focus();
        }}>{example} <span>↗</span></button
      >{/each}
  </div>
  <div
    class={css({
      textStyle: 'caption',
      color: 'fg.muted',
      mt: '6',
      display: 'flex',
      gap: '2',
      alignItems: 'center'
    })}
  >
    <span class={css({ w: '1.5', h: '1.5', rounded: 'full', bg: 'accent', flexShrink: '0' })}
    ></span> Your pace. Your path. Always another question.
  </div>
</section>
