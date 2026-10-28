import { defineConfig } from '@pandacss/dev';
import { semanticTokens, textStyles, tokens } from './src/lib/theme/tokens';

export default defineConfig({
  presets: ['@pandacss/preset-base', '@pandacss/preset-panda'],
  preflight: true,
  strictTokens: true,
  include: ['./src/**/*.{js,ts,svelte}'],
  exclude: [],
  outdir: 'styled-system',
  importMap: '#styled',
  theme: {
    extend: {
      tokens,
      semanticTokens,
      textStyles,
      breakpoints: { workspace: '64rem' }
    }
  },
  conditions: {
    extend: { coarsePointer: '@media (pointer: coarse)' }
  },
  globalCss: {
    html: {
      fontFamily: 'body',
      color: 'fg',
      bg: 'bg',
      fontSynthesis: 'none',
      colorScheme: 'light'
    },
    body: { margin: '0' },
    'button, input, textarea, select': { font: 'inherit' },
    'button, a, input, textarea': { touchAction: 'manipulation' },
    button: { cursor: 'pointer', _disabled: { cursor: 'default', opacity: '0.5' } },
    a: { color: 'accent' },
    ':where(button, a, input, textarea, select, summary):focus-visible': {
      outline: '2px solid token(colors.focus)',
      outlineOffset: '3px'
    }
  }
});
