import { defineConfig } from '@pandacss/dev';
import { semanticTokens, textStyles, tokens } from './src/lib/theme/tokens';
import { buttonRecipe } from './src/lib/theme/button.recipe';

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
      recipes: { button: buttonRecipe },
      keyframes: {
        branchEnter: {
          from: { opacity: 0, transform: 'translateY(4px)' },
          to: { opacity: 1, transform: 'none' }
        }
      },
      breakpoints: { workspace: '64rem' }
    }
  },
  conditions: {
    extend: { coarsePointer: '@media (pointer: coarse)' }
  },
  globalFontface: {
    'DM Sans': {
      src: "url('/fonts/dm-sans-variable.ttf') format('truetype')",
      fontWeight: '100 1000',
      fontStyle: 'normal',
      fontDisplay: 'swap'
    },
    'Instrument Serif': [
      {
        src: "url('/fonts/instrument-serif-regular.ttf') format('truetype')",
        fontWeight: '400',
        fontStyle: 'normal',
        fontDisplay: 'swap'
      },
      {
        src: "url('/fonts/instrument-serif-italic.ttf') format('truetype')",
        fontWeight: '400',
        fontStyle: 'italic',
        fontDisplay: 'swap'
      }
    ]
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
