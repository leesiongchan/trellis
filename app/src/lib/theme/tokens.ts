import { defineSemanticTokens, defineTextStyles, defineTokens } from '@pandacss/dev';

export const tokens = defineTokens({
  colors: {
    plum: {
      50: { value: '#fbf4f8' },
      100: { value: '#f1e2eb' },
      200: { value: '#d9b9c9' },
      300: { value: '#c9a0b7' },
      400: { value: '#b486a1' },
      500: { value: '#956580' },
      600: { value: '#7b526c' },
      700: { value: '#704a65' },
      800: { value: '#532d50' },
      900: { value: '#3a153d' },
      950: { value: '#260c29' }
    },
    peach: {
      50: { value: '#fffaf5' },
      100: { value: '#fff0e5' },
      200: { value: '#ffe1ce' },
      300: { value: '#ffcaac' },
      400: { value: '#f7aa83' },
      500: { value: '#e78861' },
      600: { value: '#ca6240' },
      700: { value: '#a1462e' },
      800: { value: '#803a2b' },
      900: { value: '#653126' },
      950: { value: '#37170f' }
    },
    cobalt: {
      50: { value: '#eef2ff' },
      100: { value: '#dfe6ff' },
      200: { value: '#c5d1ff' },
      300: { value: '#9badff' },
      400: { value: '#7185fa' },
      500: { value: '#4965f0' },
      600: { value: '#244de0' },
      700: { value: '#1c3bb7' },
      800: { value: '#1d3291' },
      900: { value: '#1c2d73' },
      950: { value: '#141c45' }
    },
    lilac: {
      50: { value: '#faf6ff' },
      100: { value: '#eee4ff' },
      200: { value: '#ddcbfa' },
      300: { value: '#c6a9ef' },
      400: { value: '#ac81df' },
      500: { value: '#925fc8' },
      600: { value: '#7b43ac' },
      700: { value: '#66368c' },
      800: { value: '#522d70' },
      900: { value: '#43265a' },
      950: { value: '#2b153d' }
    },
    coral: {
      50: { value: '#fff4f0' },
      100: { value: '#ffe3d9' },
      200: { value: '#ffc7b8' },
      300: { value: '#ffa18b' },
      400: { value: '#ff806b' },
      500: { value: '#ef5c49' },
      600: { value: '#d43d31' },
      700: { value: '#b12d28' },
      800: { value: '#902925' },
      900: { value: '#772724' },
      950: { value: '#40100e' }
    },
    apricot: {
      50: { value: '#fff8ee' },
      100: { value: '#ffeccf' },
      200: { value: '#ffdaad' },
      300: { value: '#ffc078' },
      400: { value: '#f5a34b' },
      500: { value: '#e58628' },
      600: { value: '#c76a1b' },
      700: { value: '#a34f19' },
      800: { value: '#843f1c' },
      900: { value: '#6c351b' },
      950: { value: '#3c190a' }
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
  sizes: { prose: { value: '70ch' }, control: { value: '2.75rem' }, viewport: { value: '100dvh' } },
  radii: { panel: { value: '0.875rem' }, control: { value: '0.5rem' } },
  durations: { fast: { value: '150ms' }, normal: { value: '200ms' } },
  easings: { standard: { value: 'cubic-bezier(0.2, 0, 0, 1)' } }
});

export const semanticTokens = defineSemanticTokens({
  colors: {
    fg: {
      DEFAULT: { value: '{colors.plum.900}' },
      muted: { value: '{colors.plum.700}' },
      subtle: { value: '{colors.plum.600}' }
    },
    bg: {
      DEFAULT: { value: '{colors.peach.100}' },
      canvas: { value: '{colors.lilac.100}' },
      surface: { value: '{colors.peach.50}' },
      subtle: { value: '{colors.peach.200}' },
      highlight: { value: '{colors.apricot.200}' }
    },
    border: {
      DEFAULT: { value: '{colors.plum.200}' },
      strong: { value: '{colors.plum.500}' }
    },
    accent: {
      DEFAULT: { value: '{colors.cobalt.600}' },
      hover: { value: '{colors.cobalt.700}' },
      subtle: { value: '{colors.lilac.100}' }
    },
    ornament: { value: '{colors.coral.400}' },
    focus: { value: '{colors.cobalt.700}' },
    danger: {
      DEFAULT: { value: '{colors.red.700}' },
      subtle: { value: '{colors.red.50}' }
    },
    success: { value: '{colors.emerald.700}' },
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
