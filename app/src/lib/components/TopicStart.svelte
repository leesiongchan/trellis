<script lang="ts">
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

<section class="start-section">
  <div class="eyebrow"><span class="tiny-leaf">↗</span> FOLLOW A QUESTION SOMEWHERE NEW</div>
  <h1>A little curiosity.<br />An <em>entire world.</em></h1>
  <p class="hero-description">
    Start with something you wonder about. Understand it deeply.<br class="desktop-break" /> Then follow
    the branches to whatever catches your eye.
  </p>
  <form class="topic-form" onsubmit={submit}>
    <label for="topic-input">What would you like to understand?</label>
    <div class="topic-input-wrap">
      <span class="input-spark" aria-hidden="true">✳</span><input
        id="topic-input"
        bind:this={input}
        bind:value={question}
        maxlength="500"
        placeholder="How can a tiny card hold so much?"
        autocomplete="off"
        required
        {disabled}
      /><button
        class="start-button"
        aria-label="Start exploring"
        type="submit"
        disabled={disabled || !question.trim()}>Explore <span aria-hidden="true">↗</span></button
      >
    </div>
  </form>
  <div class="examples">
    <span>A few starting points</span>{#each examples as example}<button
        onclick={() => {
          question = example;
          input.focus();
        }}>{example} <span>↗</span></button
      >{/each}
  </div>
  <div class="start-footnote">
    <span class="green-dot"></span> Your pace. Your path. Always another question.
  </div>
</section>
