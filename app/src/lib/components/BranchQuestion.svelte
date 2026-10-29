<script lang="ts">
  import { css } from '#styled/css';
  import { button } from '#styled/recipes';
  let { onFollow, disabled = false }: { onFollow: (question: string) => void; disabled?: boolean } =
    $props();
  let question = $state('');
  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!disabled && question.trim()) {
      onFollow(question.trim());
      question = '';
    }
  }
</script>

<form
  class={css({
    display: 'grid',
    gridTemplateColumns: 'minmax(0, 1fr)',
    minW: '0',
    gap: '3',
    py: '4'
  })}
  onsubmit={submit}
>
  <label class={css({ fontFamily: 'heading', fontSize: '2xl', color: 'accent' })} for="branch-input"
    >A question of your own?</label
  >
  <div
    class={css({
      display: 'flex',
      minW: '0',
      alignItems: 'center',
      gap: '2',
      p: '2',
      pl: '3',
      borderWidth: '1px',
      borderColor: 'border.strong',
      rounded: 'panel',
      bg: 'bg',
      _focusWithin: {
        outlineWidth: '2px',
        outlineStyle: 'solid',
        outlineColor: 'focus',
        outlineOffset: '1'
      }
    })}
  >
    <input
      class={css({
        flex: '1',
        minW: '0',
        minH: 'control',
        fontSize: 'md',
        bg: 'transparent',
        border: 'none',
        _placeholder: { color: 'fg.muted' },
        _focusVisible: { outline: 'none' }
      })}
      id="branch-input"
      aria-label="Your branch question"
      bind:value={question}
      maxlength="500"
      placeholder="What else are you wondering?"
      {disabled}
      required
    /><button
      type="submit"
      class={button({ visual: 'primary' })}
      aria-label="Add branch"
      disabled={disabled || !question.trim()}>↗</button
    >
  </div>
</form>
