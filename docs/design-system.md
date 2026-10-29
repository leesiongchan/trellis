# Trellis color system

The app shares the product page’s vivid botanical palette: warm peach, deep plum, cobalt, lilac, coral and apricot. Keep reading surfaces calm and reserve saturated colors for actions and small accents.

`app/src/lib/theme/tokens.ts` owns the primitive scales and semantic roles. Components use semantic roles rather than hue names or hex values.

| Role                           | Use                                                                 |
| ------------------------------ | ------------------------------------------------------------------- |
| `fg`, `fg.muted`, `fg.subtle`  | Plum text, supporting copy and metadata                             |
| `bg`, `bg.surface`             | Peach page and warm white reading surfaces                          |
| `bg.canvas`                    | Lilac map background                                                |
| `bg.subtle`                    | Quiet peach controls and grouped content                            |
| `bg.highlight`                 | Apricot root topics and explanatory callouts                        |
| `accent`, `accent.hover`       | Cobalt actions and links                                            |
| `accent.subtle`                | Lilac selections and hover surfaces                                 |
| `ornament`                     | Coral decoration; never text, controls or status information        |
| `border`, `border.strong`      | Quiet separators and identifiable control boundaries                |
| `focus`                        | Visible keyboard focus                                              |
| `danger`, `success`, `warning` | Error, success and warning states; preserve their distinct meanings |

Use plum text on light surfaces and `bg.surface` text on solid action buttons. Coral and apricot are not link colors. Selection and errors also need structural or textual cues, such as the selected border, `aria-current`, or an explicit failure message.

Keep typography, spacing, node geometry and motion in the existing theme. After a theme change, run `pnpm --dir app styles:codegen`, inspect `app/styled-system/specs/design-system.json` and the generated token types, then run `pnpm --dir app check`. The generated spec contains the resolved values; do not hand-maintain a second palette table here. `site/styles.css` mirrors the approved palette for the standalone GitHub Pages product page; the app document metadata and inline favicon also need to match when those brand colors change.
