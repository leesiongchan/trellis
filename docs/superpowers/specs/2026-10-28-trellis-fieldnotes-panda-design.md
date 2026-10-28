# Trellis Fieldnotes and Panda CSS design

Status: approved by the user on 2026-10-28; ready for implementation planning. Fieldnotes was selected after comparing three interactive directions.

Builder check: a fresh frontend reviewer found no important unanswered questions before implementation planning.

## Intent and confirmed decisions

Trellis should help someone feel productive and remain willing to continue exploring. Understanding a topic, retaining the path through it, and finding a worthwhile next question are the core outcomes.

The user requires Panda CSS and strict adherence to its philosophy and concepts, using modern practices and idioms. They selected **A / Fieldnotes** after comparing interactive mockups of three directions. Fieldnotes balances a visible branching map with an inviting, substantial reading area.

This is a frontend styling and presentation project. It replaces the existing global application stylesheet with an idiomatic Panda system and applies the selected visual direction across the application. It preserves the server-only architecture and existing research/library behavior.

## Current state and scope

The app uses Svelte 5, SvelteKit 3, Vite+, and SvelteFlow. About 1,579 lines of `app/src/app.css` currently style the root page and 14 UI components. The graph also imports SvelteFlow's distributed CSS and supplies edge/background styling through its public APIs. Panda is not installed yet.

In scope:

- Panda configuration, official Vite integration, generated types/styles, explicit generation for checks, and compatible Panda linting.
- The start screen, header, saved explorations drawer, topic catalog, retained-topic reader, workspace, map/nodes/controls, explanation/citations, suggested branches, saved connections, version history, and storage/purge UI.
- Responsive layouts, interaction states, focus, motion preferences, and long-content handling across those surfaces.
- Necessary adjustments to existing UI/browser tests and build/release checks.
- Maintainer guidance for the theme, token/recipe conventions, generated artifacts, and official Panda skills.

Out of scope:

- Backend, database, API, cookie, protocol, or research-output schema changes.
- Accounts, collaboration, quizzes, mastery scores, streaks, onboarding, or new research features.
- New generated summaries, new illustration assets, or additional model/provider calls.
- A dark-mode control, multiple brand themes, resizable panes, or a new full-map focus mode. These can be separate projects.
- Unrelated dependency upgrades or committing earlier uncommitted work.

## Approaches considered

### A / Fieldnotes — selected

Give the map and explanation stable, adjacent areas on a wide screen. Use warm paper surfaces, green accents, clear editorial typography, restrained dividers, and comfortable spacing. Keep the topic path and continuation actions easy to find.

Tradeoff: the map has less uninterrupted space than Atlas. This is the best balance between maintaining context and spending time understanding a topic. The user selected it.

### B / Atlas — considered

Give the map the main canvas and place the explanation in a floating card. This favors spatial discovery and makes the branching structure more prominent.

Tradeoff: a smaller reading area and more competing spatial elements make long explanations less comfortable. Not selected.

### C / Reading desk — considered

Give the explanation most of the screen with a compact topic outline and a separate full-map view. This favors sustained reading.

Tradeoff: less of the branching structure is visible while reading, and switching views adds navigation. Not selected.

The throwaway comparison lives at `.superpowers/brainstorm/3988-1793185376/content/trellis-exploration-directions.html`. It is visual reference, not production code or researched content.

## Assistant-proposed defaults

These assistant-proposed defaults were accepted with design approval; they were not additional requirements stated independently by the user:

- Apply Fieldnotes to all existing frontend surfaces, including the start screen and saved-library interfaces.
- Ship one light appearance with semantic tokens that make future changes practical.
- Keep the existing map/read interaction model, existing research actions, and existing view persistence. Do not add the prototype's map-focus action.
- Use the app's existing font families and reliable system fallbacks. Add no external font request or font dependency in this work.
- Use complete, cited explanations as the content source. The prototype's reading estimates, citation markers, and “connection to keep” copy are illustrative and do not introduce production fields or generated claims.
- Show honest progress through existing saved state, selected nodes, and the visible path. Do not introduce a new counter, learning metric, or completion claim.

## Experience and visual structure

### Start and return

The first screen gives the question input priority, keeps optional starting points, and makes saved explorations easy to resume. Retain Trellis branding and the current supporting copy unless a small wording adjustment is necessary for clarity. Apply the Fieldnotes type, surface, spacing, and control vocabulary instead of introducing a second visual identity for the start screen.

Saved explorations and topics use readable titles, dates and existing metadata. Long questions wrap without forcing horizontal overflow. Destructive actions remain explicit and confirmed. Saving, pending work, conflicts, disconnection, and recovery remain distinguishable and truthful.

### Desktop exploration

- Keep a compact application header and a topic-path toolbar above the workspace.
- Start from a near-balanced map/reader split, giving the reader slightly more space as shown in Fieldnotes. Exact grid ratios and the point of switching layouts belong to implementation and content testing.
- Give the reader sufficient width for real multi-section explanations. It scrolls independently while map context stays available.
- Use a bounded prose measure, approximately 60–75 characters where space allows; let narrower layouts reflow naturally.
- Keep select and expand/collapse as separate node actions. Selected, suggested, researching, ready, and failed states remain distinguishable without relying only on color.
- Preserve the current graph layout algorithm and saved viewport behavior. Style graph edges, background, and library-owned elements through SvelteFlow's supported properties/CSS variables wherever possible.
- Preserve existing root, refocus, collapse, pan, zoom, and ancestry controls. Do not automatically rearrange or recenter the map on every update.

### Reader and continuation

Use a strong heading, clear opening answer, readable sections, inline citations, and an accessible source list. Existing analogies and limitations can use a quiet inset surface. Keep evidence associated with the paragraphs and connections it supports.

Retain all branch suggestions and custom questions. Suggested branches show their existing reasons so that a person can judge why they might follow one. Retain saved-topic connections, evidence, dismissal, pagination, and version choice; group these secondary capabilities clearly without hiding necessary state or adding a new navigation layer.

Loading, clarification, retry, refresh, and retained-topic modes receive the same care as the successful reader. Refresh continues to show the earlier answer until replacement succeeds. Saved-topic reading and starting a separate exploration remain visibly different actions.

### Narrow screens and zoom

At the width where both panes stop being usable, retain the existing explicit Map/Read switch. Both views refer to the same selected topic and persisted exploration state. Returning to the map retains its context; selecting a topic follows existing navigation behavior.

Use fluid spacing and bounded type scales instead of a series of unrelated pixel overrides. Use container queries when a component's own space determines its arrangement; use media queries for overall workspace modes and user preferences. Exact thresholds are implementation calls, justified by real content rather than device names.

The page, reader, navigation, forms, and library surfaces must work at 320 CSS pixels and with enlarged text/zoom. The graph remains a two-dimensional viewport with independent navigation; surrounding UI must reflow normally. Long titles, URLs, breadcrumbs, error messages, citations, and confirmation phrases must not break controls or obscure focus.

## Panda architecture

Follow the official v2 guidance and the official `panda-css` and `panda-css-theming` skills. Confirm current compatible package versions when implementing and use caret ranges consistent with this repository.

### Theme vocabulary

- Extend Panda's base and Panda presets rather than rebuilding the utility system.
- Define primitive scales for the palette, spacing, sizes, radii, type, and motion values that need customization. Keep fluid `clamp()` expressions in theme definitions.
- Define semantic tokens for shared roles such as foreground, secondary text, canvas, surfaces, boundaries, accent, focus, and feedback. Names in this document are conceptual; choose and verify the actual generated token names during implementation.
- Use composite text styles where typography properties form a recurring named unit. Do not duplicate type scales in individual components.
- Enable `strictTokens`. Avoid raw-value escapes in components; document any necessary exception at the call site. CSS keywords and genuinely computed graph geometry are not invented theme tokens.
- Keep global styling to intentional document/reset rules, shared accessibility defaults, and narrowly justified third-party integration. Do not move the old component selectors wholesale into `globalCss`.

### Component styles and extraction

- Put one-off styles next to their Svelte markup with `css()`. Use Panda patterns for useful layout compositions.
- Use `cva` when a component actually has a meaningful variant API. Use `sva` when the same variants coordinate multiple styled slots.
- Promote recipes to configuration only when they are shared design-system primitives or need that generation model. Avoid a config recipe for every element and avoid one enormous page recipe.
- Keep all style choices statically extractable. Runtime state selects known literal styles/variants; genuinely computed values use supported CSS variables or library APIs.
- Verify runtime-selected config-recipe variants are emitted. Do not silence extraction warnings with broad generation of every possible style.
- Use modern Svelte class composition and reactive derived values for state-dependent recipe classes. Preserve SvelteFlow's required behavioral classes such as `nodrag` and `nopan`.
- Import runtime helpers from the generated output and config helpers from `@pandacss/dev`. Never hand-edit generated output.

### Build, layers, and enforcement

- Integrate the official Panda Vite plugin with the existing Vite+ / SvelteKit setup and unit-test branch. Preserve the current adapter, aliases, CSP, and dev proxy behavior.
- Declare the intended cascade layer order. Scope any SvelteFlow adaptation carefully so unlayered vendor rules do not unpredictably override application styling.
- Generate artifacts explicitly before checks/editor-dependent tooling. CI currently installs with `--ignore-scripts`; do not depend solely on a package lifecycle hook. Keep generated output ignored and reproducible from a clean checkout.
- Include every styled Svelte/TypeScript source. Check generated token types and the resolved design-system spec after theme changes.
- Integrate Panda's supported Oxlint rules with Vite+ if compatible. Verify Svelte coverage explicitly; if the integration cannot inspect `.svelte` source, add a narrowly scoped supported check for that coverage rather than claiming the files are linted.
- Enable source transforms where supported and verified in the actual Svelte build. CSS is generated at build time; do not introduce runtime style injection. If a class-resolution helper remains for stateful recipes, document that distinction rather than claiming all styling JavaScript was removed.
- Preserve pinned tool runtimes and the repository's disabled arbitrary dependency install scripts.

## Accessibility, motion, and performance

Aim for WCAG 2.2 AA, with strong focus visibility and roughly 44px targets for primary touch controls. Verify text contrast, distinguish selection from focus, preserve semantic buttons/links, and keep actions usable with a keyboard. Sources and compact citations must remain operable without hover.

Use motion to explain a state transition or maintain spatial continuity. Respect `prefers-reduced-motion` for both CSS effects and JavaScript viewport movement. Avoid perpetual decorative motion, broad `transition: all`, and animations that delay input or hide useful content. Exact timing belongs to implementation and inspection.

Keep loading and errors honest; do not invent percentage progress for provider work. Preserve the distinction between application interaction latency and external research latency.

Measure representative rendering and interaction behavior with real fixtures. Core Web Vitals targets are LCP ≤2.5s, INP ≤200ms, and CLS ≤0.1 at the 75th percentile when sufficient field measurements exist. Local measurements are diagnostic evidence and must not be presented as field compliance. Do not add telemetry services for this migration.

## Verification and completion

- `pnpm check` passes, including Svelte diagnostics, formatting, lint and types, with explicit Panda generation/checks.
- Existing frontend unit/integration tests pass. Add focused coverage only for meaningful new presentation logic or behavior adjustments.
- The production frontend and packaged release build correctly from a clean generated-artifact state; release manifest/CSP verification passes.
- Existing browser acceptance passes, including persistence, lost-response recovery, multiple tabs, connection navigation, retained topics, versions, purge confirmation, legacy cookies, and HTTP/HTTPS fixture-backed research.
- Visually inspect the start screen, active workspace, saved topics/explorations, retained reader, and error/loading states at representative wide, intermediate, and phone widths. Check long content, keyboard navigation, zoom/text enlargement, and reduced motion.
- Inspect emitted CSS for recipe/state coverage, token resolution, duplicated styling, and remaining legacy selectors. Remove the old application component rules; leave only justified global rules, the layer declaration, and supported vendor styling.
- Verify ordinary navigation/restyling does not initiate extra paid research. No live paid-provider calls are needed for acceptance.
- Backend logic and stored formats stay unchanged; use existing browser/release verification for integration rather than inventing backend tests for a styling migration.

## Research and references

- [Thinking in Panda](https://panda-css.com/docs/get-started/thinking-in-panda)
- [Official Panda styling skill](https://github.com/chakra-ui/panda/blob/v2/skills/panda-css/SKILL.md)
- [Official Panda theming skill](https://github.com/chakra-ui/panda/blob/v2/skills/panda-css-theming/SKILL.md)
- [Panda Svelte integration](https://panda-css.com/docs/get-started/svelte)
- [Panda linting](https://panda-css.com/docs/get-started/linting)
- [Utopia fluid design](https://utopia.fyi/)
- [MDN container queries](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Containment/Container_queries)
- [Progressive disclosure](https://www.nngroup.com/articles/progressive-disclosure/)
- [Animation for attention and comprehension](https://www.nngroup.com/articles/animation-usability/)
- [WCAG 2.2 understanding documents](https://www.w3.org/WAI/WCAG22/Understanding/)
- [Core Web Vitals](https://web.dev/articles/vitals)

## Left to implementation planning

Exact token names/scales, component boundaries, recipe anatomy and variant axes, fluid bounds, workspace thresholds, plugin ordering, lint wiring, source-transform coverage, and the sequence of component migrations are builder decisions within the constraints above. Keep these choices small and verify them against generated output and actual rendered content.
