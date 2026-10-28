import { defineRecipe } from '@pandacss/dev';

export const buttonRecipe = defineRecipe({
  className: 'control',
  base: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '2',
    borderWidth: '1px',
    borderColor: 'transparent',
    rounded: 'control',
    fontSize: 'sm',
    fontWeight: 'medium',
    lineHeight: 'tight',
    textAlign: 'center',
    cursor: 'pointer',
    transitionProperty: 'color, background-color, border-color',
    transitionDuration: 'fast',
    transitionTimingFunction: 'standard',
    _focusVisible: {
      outlineWidth: '2px',
      outlineStyle: 'solid',
      outlineColor: 'focus',
      outlineOffset: '1'
    },
    _disabled: { opacity: '0.5', cursor: 'default' },
    _motionReduce: { transition: 'none' }
  },
  variants: {
    visual: {
      primary: { bg: 'accent', color: 'bg.surface', _hover: { bg: 'accent.hover' } },
      quiet: { bg: 'transparent', color: 'accent', _hover: { bg: 'accent.subtle' } },
      outline: {
        bg: 'bg.surface',
        color: 'accent',
        borderColor: 'border.strong',
        _hover: { bg: 'accent.subtle', borderColor: 'accent' }
      },
      danger: { bg: 'danger', color: 'bg.surface', _hover: { opacity: '0.9' } },
      icon: { bg: 'transparent', color: 'accent', minW: 'control', _hover: { bg: 'accent.subtle' } }
    },
    size: {
      sm: { minH: '8', px: '3', py: '1', _coarsePointer: { minH: 'control' } },
      md: { minH: 'control', px: '4', py: '2' }
    }
  },
  compoundVariants: [{ visual: 'icon', css: { px: '2' } }],
  defaultVariants: { visual: 'quiet', size: 'md' }
});
