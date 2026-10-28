import { defineSemanticTokens, defineTextStyles, defineTokens } from '@pandacss/dev';

export const tokens = defineTokens({
  colors: {
    forest: {
      50: { value: '#f0f5ec' },
      100: { value: '#e7efdf' },
      200: { value: '#cedec2' },
      300: { value: '#b0c49e' },
      400: { value: '#96ae81' },
      500: { value: '#7b9666' },
      600: { value: '#607e51' },
      700: { value: '#4f6d48' },
      800: { value: '#3e6547' },
      900: { value: '#26392c' },
      950: { value: '#17271d' }
    },
    sand: {
      50: { value: '#fffef9' },
      100: { value: '#f7f7f2' },
      200: { value: '#eef0e7' },
      300: { value: '#e0e4d8' },
      400: { value: '#c7cdbd' },
      500: { value: '#a5ad99' },
      600: { value: '#818b75' },
      700: { value: '#606a55' },
      800: { value: '#454d3d' },
      900: { value: '#2d3529' },
      950: { value: '#1c2219' }
    }
  },
  fonts: {
    body: { value: "'DM Sans', system-ui, sans-serif" },
    heading: { value: "'Instrument Serif', Georgia, serif" }
  },
  fontSizes: {
    body: { value: 'clamp(1rem, 0.94rem + 0.2vw, 1.125rem)' },
    topic: { value: 'clamp(2rem, 1.4rem + 2vw, 3rem)' },
    hero: { value: 'clamp(3rem, 2rem + 4vw, 5rem)' }
  },
  spacing: {
    gutter: { value: 'clamp(1rem, 0.6rem + 1.5vw, 2.5rem)' },
    section: { value: 'clamp(1.5rem, 1rem + 2vw, 3rem)' }
  },
  sizes: { prose: { value: '70ch' }, control: { value: '2.75rem' } },
  radii: { panel: { value: '0.875rem' }, control: { value: '0.5rem' } },
  durations: { fast: { value: '150ms' }, normal: { value: '200ms' } },
  easings: { standard: { value: 'cubic-bezier(0.2, 0, 0, 1)' } }
});

export const semanticTokens = defineSemanticTokens({
  colors: {
    fg: {
      DEFAULT: { value: '{colors.forest.900}' },
      muted: { value: '{colors.sand.700}' },
      subtle: { value: '{colors.gray.600}' }
    },
    bg: {
      DEFAULT: { value: '{colors.sand.100}' },
      canvas: { value: '{colors.forest.50}' },
      surface: { value: '{colors.sand.50}' },
      subtle: { value: '{colors.sand.200}' }
    },
    border: {
      DEFAULT: { value: '{colors.sand.300}' },
      strong: { value: '{colors.sand.600}' }
    },
    accent: {
      DEFAULT: { value: '{colors.forest.800}' },
      hover: { value: '{colors.forest.900}' },
      subtle: { value: '{colors.forest.100}' }
    },
    focus: { value: '{colors.amber.700}' },
    danger: {
      DEFAULT: { value: '{colors.red.700}' },
      subtle: { value: '{colors.red.50}' }
    },
    success: { value: '{colors.forest.800}' },
    warning: { value: '{colors.amber.800}' }
  }
});

export const textStyles = defineTextStyles({
  body: {
    DEFAULT: { value: { fontFamily: 'body', fontSize: 'body', lineHeight: '1.8' } },
    sm: { value: { fontFamily: 'body', fontSize: 'sm', lineHeight: '1.7' } }
  },
  heading: {
    hero: {
      value: {
        fontFamily: 'heading',
        fontSize: 'hero',
        lineHeight: '1.06',
        letterSpacing: '-0.04em',
        fontWeight: 'normal'
      }
    },
    topic: {
      value: {
        fontFamily: 'heading',
        fontSize: 'topic',
        lineHeight: '1.12',
        letterSpacing: '-0.035em',
        fontWeight: 'normal'
      }
    },
    section: {
      value: { fontFamily: 'body', fontSize: 'lg', lineHeight: '1.4', fontWeight: 'semibold' }
    }
  },
  label: {
    value: {
      fontFamily: 'body',
      fontSize: 'xs',
      fontWeight: 'semibold',
      letterSpacing: '0.1em',
      textTransform: 'uppercase',
      lineHeight: '1.5'
    }
  },
  caption: { value: { fontFamily: 'body', fontSize: 'xs', lineHeight: '1.6' } }
});
