<script lang="ts">
  import { css } from '#styled/css';
</script>

<div
  class={css({
    display: 'none',
    workspace: { display: 'grid' },
    alignContent: 'center',
    gap: '5',
    minW: '0'
  })}
  aria-hidden="true"
>
  <img
    class={css({ w: 'full', h: 'auto', rounded: '4xl', mixBlendMode: 'multiply' })}
    src="/images/botanical-canopy.webp"
    alt=""
    width="1448"
    height="1086"
    decoding="async"
  />
  <p
    class={css({ fontFamily: 'heading', fontSize: '2xl', textAlign: 'center', color: 'fg.muted' })}
  >
    One thing leads to another.
  </p>
</div>
