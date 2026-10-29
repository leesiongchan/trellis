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

## Living field guide

Use DM Sans for prose and controls, and Instrument Serif for editorial headings. Both families ship locally in `app/static/fonts/` with their licenses; Panda's `globalFontface` owns the declarations. Keep system fallbacks and `font-display: swap`. The body scale starts at 1.125rem, ordinary controls at 1rem and supporting copy at 0.875rem. Text scales with the user's root size.

Use the existing canopy artwork on the start screen, with a correct intrinsic aspect ratio and empty alt text. It is decoration, never a source of labels or interactive questions. Keep it away from the phone's input flow. Use `BotanicalMark` for small decorative leaves and sparks; avoid a second illustration style or status encoded in ornament color.

The live map retains its fixed 240 × 150 CSS-pixel cards and separate read/expand controls. Prioritize readable relationship/status and title text over teaser copy. Cobalt boundaries indicate selection; keyboard focus remains independently visible. Curved edges and a slightly asymmetric apricot root reinforce the branching structure without changing layout or saved viewport data.

Use one brief entrance response for a new node and restrained color/boundary transitions. Disable these with reduced motion, and pass an explicit zero duration to map zoom, fit and refocus actions. Do not use continuous drifting, pulsing, or simulated research progress.

Continuation and saved-exploration cards respond to their available width through container conditions. Decorative marks yield space to text on narrow or enlarged layouts. The phone workspace has a minimum usable height and can extend the document when the header grows; the wide workspace retains independent map/reader scrolling. Reader grids use bounded columns so long labels and controls cannot expand the pane horizontally.
