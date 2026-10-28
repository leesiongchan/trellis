# Trellis Fieldnotes and Panda CSS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Trellis's global component styling with idiomatic Panda CSS and deliver the approved fluid Fieldnotes experience across the existing app.

**Architecture:** Keep the existing Svelte 5 components, server session, and SvelteFlow navigation model. Generate a strict, semantic Panda theme in `app/styled-system`, colocate application styles with their Svelte markup, and use a shared button recipe only where controls share a real variant API. Migrate complete surfaces sequentially and remove their legacy rules as each surface moves.

**Tech Stack:** Svelte 5, SvelteKit 3, Vite+ 1.4, Panda CSS 2, TypeScript 7 plus the existing TypeScript 6 API alias, Playwright, existing Rust fixture/release servers.

**Spec:** `docs/superpowers/specs/2026-10-28-trellis-fieldnotes-panda-design.md` (approved 2026-10-28).

## Global Constraints

- “It preserves the server-only architecture and existing research/library behavior.”
- “Backend, database, API, cookie, protocol, or research-output schema changes” are out of scope.
- “Ship one light appearance with semantic tokens that make future changes practical.”
- “Use the app's existing font families and reliable system fallbacks. Add no external font request or font dependency in this work.”
- “Do not introduce a new counter, learning metric, or completion claim.” Prototype estimates, takeaways, and map-focus mode do not become features.
- “Enable `strictTokens`.” Read generated token types; document the rare necessary raw-value escape. Extend both Panda presets.
- “The page, reader, navigation, forms, and library surfaces must work at 320 CSS pixels and with enlarged text/zoom.” The graph remains an independently navigable two-dimensional viewport.
- “Aim for WCAG 2.2 AA, with strong focus visibility and roughly 44px targets for primary touch controls.”
- Preserve existing research actions, recovery, cross-tab fencing, versions, retained topics, citations, confirmations, and persisted selection/viewport behavior.
- Use caret versions for new dependencies. Preserve Vite+ aliases, tool runtime pins, CSP, adapter, disabled dependency install scripts, and all earlier uncommitted work.
- This plan is a frontend project. Use fixture-backed acceptance, not paid provider calls or real-library mutation.

## Review Focus

1. Clean installs with `--ignore-scripts`: checks and builds must regenerate Panda artifacts without relying on a lifecycle hook (Task 1).
2. Long questions, citation URLs, and confirmation text at 320px or enlarged text: controls remain operable and surrounding UI does not overflow (Tasks 2 and 4).
3. Runtime selection, research, refresh/error, and mobile-pane states: their styles are actually emitted and content/recovery remain truthful (Tasks 3 and 4).
4. Keyboard focus and reduced motion: citations and graph controls stay operable, visible, and free from unwanted viewport motion (Tasks 3 and 4).
5. Vendor/reset/cascade interactions: SvelteFlow controls, node geometry, selection, zoom, and persisted view remain correct after legacy CSS removal (Tasks 1, 3, and 5).

---

## Workspace and file responsibilities

The current workspace contains uncommitted dependency updates, Tailscale documentation cleanup, and the Trellis rename. The approved spec is committed separately as `b20e5f3` plus its approval annotation. Record `git status --short` and preserve those changes. If isolation is required, use the worktree skill and carry the exact working state; a clean checkout of HEAD alone omits authorized dependency and rename work. Never reset, discard, or include earlier changes in migration commits accidentally. Commit only migration-owned files/hunks; when separating shared-file hunks is unsafe, retain those edits uncommitted and report that fact.

New files:

- `app/panda.config.ts`: presets, extraction paths, generated output, theme composition, and global document defaults.
- `app/src/lib/theme/tokens.ts`: project primitive scales, semantic tokens, text styles, and restrained motion definitions consumed only by Panda config.
- `app/src/lib/theme/button.recipe.ts`: shared control recipe consumed by Panda config.
- `app/src/lib/components/AppHeader.svelte`: extracted existing header, including home/exploration presentation and current save/library actions.
- `app/tests/e2e/fieldnotes.spec.ts`: meaningful responsive and keyboard presentation acceptance using the real fixture server.
- `app/AGENTS.md`: project-specific Panda conventions and official skill references.
- Conditional only if the Vite+ integration cannot lint Svelte: `app/eslint.panda.config.mjs`, a Panda-only Svelte lint entry point.

Modify existing package manifests/lockfile, Vite/type configuration, `.gitignore`, root check wiring, `app/src/app.css`, `+layout.svelte`, `+page.svelte`, all styled UI components, existing mobile/recovery/library browser tests, and README. No backend files are required.

Generated, ignored: `app/styled-system/**`. Never hand-edit it. Retain ignored mockups as reference; do not copy their HTML, sample content, JavaScript, or raw stylesheet into production.

## Planned theme and component interfaces

These are implementation calls within the approved design, adjustable only when rendering/contrast evidence justifies it:

- Generated imports use `#styled/css`, `#styled/patterns`, `#styled/recipes`, and `#styled/tokens`. Set Panda `importMap: '#styled'` and match the existing package-import/Vite alias conventions.
- `app/src/lib/theme/tokens.ts` exports named `tokens`, `semanticTokens`, and `textStyles` objects using the corresponding typed Panda config helpers. It does not import generated runtime helpers.
- Extend the inherited scales. Use a complete project forest palette with the mockup's green accent near `#3e6547`; use inherited neutral/feedback scales where suitable. Put raw color values in primitive definitions, not semantic tokens or components. Contrast is a completion gate, not assumed from the mockup.
- Shared semantic color paths: `fg`, `fg.muted`, `fg.subtle`, `bg`, `bg.canvas`, `bg.surface`, `bg.subtle`, `border`, `border.strong`, `accent`, `accent.hover`, `accent.subtle`, `focus`, `danger`, `danger.subtle`, `success`, `warning`. Define every name before use and verify generated types.
- Fonts: `body` uses the existing DM Sans/system stack; `heading` uses the existing Instrument Serif/Georgia stack. Text styles: `body`, `body.sm`, `heading.hero`, `heading.topic`, `heading.section`, `label`, `caption`.
- Starting fluid bounds: body `clamp(1rem, 0.94rem + 0.2vw, 1.125rem)`; topic heading `clamp(2rem, 1.4rem + 2vw, 3rem)`; hero `clamp(3rem, 2rem + 4vw, 5rem)`; page gutter `clamp(1rem, 0.6rem + 1.5vw, 2.5rem)`. Store these in theme definitions, retain zoom/text enlargement, and inspect actual line lengths.
- Add the `workspace: '64rem'` breakpoint as the initial Map/Read layout threshold. Start the wide workspace near a 45% map / 55% reader split. Components use container queries for their own available width; adjust the threshold only for a demonstrated content/layout problem.
- `buttonRecipe` is a named export from `button.recipe.ts`, registered as `theme.recipes.button`; generated `button({ visual?, size? })` has `visual: 'primary' | 'quiet' | 'outline' | 'danger' | 'icon'`, `size: 'sm' | 'md'`, default `quiet`/`md`. Use literal variant arguments at call sites; defaults and every used combination must emit. `md` primary targets are at least 44px. Compact controls grow for coarse pointers where necessary.
- Local components use `css()`; use local `cva`/`sva` only for actual state/slot variation. Derive state-dependent class bags with `$derived`, and use Svelte class arrays/objects or `cx` for required library classes. Do not create a global recipe per component.
- `AppHeader` consumes `exploring: boolean`, `saveState: 'saved' | 'saving' | 'failed' | 'conflict' | 'offline'`, `disabled: boolean`, `explorationCount: number`, `onHome: () => void`, `onSavedTopics: () => void`, `onExplorations: () => void`. Preserve the existing accessible names and callbacks. The root page still owns library/session state.

### Task 1: A working Panda foundation and first migrated surface

**Files:** Create `app/panda.config.ts`, `app/src/lib/theme/tokens.ts`; modify `package.json`, `app/package.json`, `pnpm-lock.yaml`, `app/vite.config.ts`, `app/tsconfig.json`, `.gitignore`, `vite.config.ts`, `app/src/app.css`, `app/src/routes/+layout.svelte`, `app/src/lib/components/TopicStart.svelte`; conditionally create `app/eslint.panda.config.mjs` for the scoped lint path described above.

**Interfaces:** Produces the generated `#styled/*` import surface and theme vocabulary above. The existing `TopicStart` props and form behavior remain unchanged. Produces `app` scripts `styles:codegen` and `styles:check`, called explicitly by app/root checks as needed.

- [x] **Step 1: Record the current passing frontend baseline.** Run `pnpm check` and `pnpm test:unit --run`. Record results and the current browser fixture/release prerequisites. Use `pnpm_config_verify_deps_before_run=false` in this environment to avoid pnpm implicitly reinstalling against a different store.
- [x] **Step 2: Install compatible Panda tooling with caret ranges and scripts disabled.** Registry research on 2026-10-28 found `@pandacss/dev`, `@pandacss/vite`, `@pandacss/preset-base`, `@pandacss/preset-panda`, and `@pandacss/eslint-plugin` all at `2.4.0`, requiring Node ≥22. Recheck metadata before install; preserve runtime pins. Install the build/theme packages in `app`, the lint plugin where root Vite+ can resolve it, and declared required peers at compatible current versions. Keep the Vite+ alias; do not override peer requirements with an incompatible Vite package.
- [x] **Step 3: Define the theme and generated imports.** Apply the interfaces above with both explicit presets, `strictTokens: true`, `preflight: true`, `include: ['./src/**/*.{js,ts,svelte}']`, `outdir: 'styled-system'`, and `importMap: '#styled'`. Add alias/import/type coverage and ignored output. Run codegen with `--spec`; read `types/tokens.d.ts` and `specs/design-system.json` rather than guessing token paths.
- [x] **Step 4: Integrate CSS generation and layers.** Add the official Vite plugin to the existing lazy SvelteKit and unit-test branches, retaining configuration behavior. Enable `transform: true` only after verifying Svelte output. Import SvelteFlow CSS once into a named vendor layer from the root CSS and remove its unlayered page import. During migration declare `reset, vendor, base, tokens, legacy, recipes, utilities`; temporarily put remaining old rules in `legacy`. The final task removes that layer. Keep document/reset defaults in config; avoid a second global component stylesheet.
- [x] **Step 5: Wire enforceable diagnostics.** `styles:codegen` runs `panda codegen --spec`; app `check` invokes it before Svelte sync/check. `styles:check` performs full Panda extraction/build diagnostics and the chosen lint path. Configure the official Oxlint plugin and explicit recommended correctness rules for app sources. Probe a temporary included `.svelte` file with an invalid token and missing `&`, confirm the chosen checks fail, then remove the probe. If Vite+ cannot inspect Svelte, use Panda's recommended ESLint rules with the supported Svelte parser in `app/eslint.panda.config.mjs`, restricted to app Svelte sources; root `check` calls that scoped command. Do not report unsupported Svelte coverage as linted. Add only the parser/peer dependencies this fallback actually requires.
- [x] **Step 6: Migrate `TopicStart` using colocated `css()`.** Preserve input, examples, disabled/submit behavior and labels. Use the fluid theme, strong hierarchy and wrapping layout. Remove its old component selectors. Verify generated CSS and actual rendered styles rather than checking only class strings.
- [x] **Step 7: Verify clean generation and the first surface.** Remove only known generated `app/styled-system` artifacts, then run `pnpm check`, `pnpm test:unit --run`, and `pnpm build:frontend`. Expected: generation succeeds without lifecycle scripts; diagnostics/types pass; production CSS contains the migrated start styles without missing-token/extraction warnings. Check desktop and 320px start input/submit accessibility. Record any transform/lint limitations concretely.
- [x] **Step 8: Commit migration-owned foundation changes.** Keep prior dependency/rename hunks separate; never stage entire shared files blindly.

### Task 2: Fieldnotes shell, start screen, and saved explorations

**Files:** Create `app/src/lib/theme/button.recipe.ts`, `app/src/lib/components/AppHeader.svelte`, `app/tests/e2e/fieldnotes.spec.ts`; modify `app/panda.config.ts`, `app/src/routes/+page.svelte`, `app/src/lib/components/TopicStart.svelte`, `app/src/lib/components/ExplorationList.svelte`, and `app/src/app.css`.

**Interfaces:** Consumes Task 1's theme/imports; produces `button()` and `AppHeader` with the exact interfaces above. Session state, navigation, BroadcastChannel identifiers and mutations stay in the existing root page.

- [x] **Step 1: Add one meaningful responsive acceptance case.** In `fieldnotes.spec.ts`, add `start and saved exploration controls reflow at 320px`. Use `observeResearch`, `seedServerExploration`, and `makeExploration` to provide a valid long saved question, without new provider calls. At 320×800, assert `document.documentElement.scrollWidth <= innerWidth`, the start input and `Start exploring` button are reachable, and the saved exploration's resume/delete confirmation actions remain visible after opening the library. Reuse this test for enlarged-text inspection; avoid brittle generated-class assertions.
- [x] **Step 2: Run the case against the currently packaged frontend.** `pnpm --dir app exec playwright test tests/e2e/fieldnotes.spec.ts --project=production`. Record baseline failures; do not invent a failure if existing behavior already passes. Browser checks always need a freshly packaged frontend after edits.
- [x] **Step 3: Implement the shared control recipe and regenerate.** Register the stated variants/defaults and focus/disabled states. Convert `TopicStart` controls to the generated recipe. Ensure literals cover the variants that consumers call; use `css()` for a genuine one-off override.
- [x] **Step 4: Extract and style `AppHeader`.** Match Fieldnotes's compact wordmark, gentle divisions, readable saved state, and clear library/new-exploration actions. Use local slot styling only where home/exploring variants coordinate elements. Wrap or rearrange actions at narrow widths; do not hide required actions to make the header fit.
- [x] **Step 5: Migrate the root page and `ExplorationList`.** Apply colocated styles to the shell, start composition/illustration, notices, drawer, empty/list/delete states, topic catalog and return-location control. Use parent-owned layout gaps and wrapping titles. Tokenize the existing inline illustration colors; add no asset or new content. Keep the page's async actions unchanged. Remove the corresponding legacy rules.
- [x] **Step 6: Rebuild, package, and verify.** Run `pnpm check`, `pnpm build:frontend`, `node scripts/package-release.ts`, then the new case and existing persistence/recovery browser specs. Expected: Trellis branding and role labels stay intact, the case passes, confirmations still gate deletion, and saving/error states remain truthful. Inspect the start/header at 320, 768, and 1440px, including enlarged text.
- [x] **Step 7: Commit only this surface's migration changes.**

### Task 3: Balanced workspace, graph states, and accessible controls

**Files:** Modify `app/src/lib/components/Workspace.svelte`, `app/src/lib/components/TreeCanvas.svelte`, `app/src/lib/components/TopicNode.svelte`, `app/src/lib/components/WorkspaceControls.svelte`, `app/src/lib/components/TopicTree.svelte` only if its wrapper needs layout adjustments, `app/tests/e2e/mobile.spec.ts`, `app/tests/e2e/fieldnotes.spec.ts`, and `app/src/app.css`.

**Interfaces:** Existing component props and `WorkspaceSession` methods stay unchanged. A local TopicNode slot recipe owns `active`, `root`, and research/status styling. It must preserve SvelteFlow's `nodrag`/`nopan` classes and existing select/expand controls. No change to `layoutTree` or its 320px depth / 174px row spacing.

- [x] **Step 1: Add `selected topic styles survive navigation and reduced-motion map controls` in `fieldnotes.spec.ts`.** Use `observeResearch`/`startExample`, enable reduced motion, keyboard-activate `Read NAND flash`, and verify the explanation plus an accessible current-state marker (`aria-current="location"` on its read button). Focus/activate refocus and verify focus remains visible. Switch away/back without extra research for already loaded content. The case must also assert rendered selected-node border/background differs from a suggested node, not just that recipe classes exist.
- [x] **Step 2: Extend the existing phone case to 320px.** Preserve selected topic across Map/Read switches and verify surrounding document overflow stays absent. Run these targeted cases before changes and record the expected missing current-state marker/any existing layout failures.
- [x] **Step 3: Migrate `Workspace` composition.** Use the planned wide split and switch threshold, keep an independent reader scroller, and use explicit local mobile-pane variants. Hide the inactive pane accessibly without losing mounted map state. Keep root/path/collapse controls and render long breadcrumbs without squeezing the Map/Read switch.
- [x] **Step 4: Migrate graph/node/control styles.** Use semantic colors through supported SvelteFlow APIs and CSS variables. Keep numeric 240×150 node geometry aligned with the existing layout and canvas props; a documented geometry exception is preferable to rem-based node dimensions that drift from the algorithm. Use local `sva` for node slots that react together to active/root/status axes. Add the current-state marker without changing node actions. Ensure primary node controls have usable hit areas within the fixed geometry. Remove old descendant selectors; retain only a scoped, justified library integration if no public hook exists.
- [x] **Step 5: Verify motion and viewport behavior.** Keep automatic focusing tied to the existing signature/restoration logic. Respect reduced motion in existing programmatic panning; do not add movement to the zoom/fit controls. Confirm that no newly introduced CSS animation runs when reduced motion is requested.
- [x] **Step 6: Rebuild/package and run `mobile.spec.ts`, `knowledge-tree.spec.ts`, `persistence.spec.ts`, and the new cases with the production project.** Expected: keyboard and touch controls work, select/expand remain distinct, generated state styles apply, and viewport/selection restore. Visually inspect graph zoom/pan, node states, and the balanced reader at 960, 1024, and 1440px.
- [x] **Step 7: Commit only workspace/graph migration changes.**

### Task 4: Readable answers and the complete saved-library experience

**Files:** Modify `app/src/lib/components/TopicReader.svelte`, `app/src/lib/components/CitationLink.svelte`, `app/src/lib/components/BranchQuestion.svelte`, `app/src/lib/components/RetainedTopic.svelte`, `app/src/lib/components/TopicConnections.svelte`, `app/src/lib/components/AnswerVersions.svelte`, `app/src/lib/components/LibraryStatus.svelte`, `app/tests/e2e/fieldnotes.spec.ts`, `app/tests/e2e/recovery.spec.ts`, `app/tests/e2e/library-graph.spec.ts`, and `app/src/app.css`.

**Interfaces:** Keep every existing prop/callback, source URL/relationship, evidence association, answer-selection action, and purge-confirmation contract. Component-local citation variants and reader/connection slots consume the shared theme/button recipe; no new content fields or backend calls.

- [x] **Step 1: Add `long cited explanations reflow and keep keyboard focus visible` in `fieldnotes.spec.ts`.** Seed valid content with a long title, long source URL/publisher, multiple cited sections and a long branch question via existing fixture builders/import helper. At 320px with Read active, assert no document horizontal overflow, citation `Source 1: …` has its original safe href/rel, keyboard navigation can reach the citation/source/custom-question controls, and each focused control has a nonzero visible focus indicator inside its scrolled reader. Confirm the primary submit hit area is at least 44px in height. Do not navigate to an external source during the test.
- [x] **Step 2: Extend existing acceptance where it owns the state.** In recovery tests, assert old cited content remains visible during a pending refresh and that a failed/disconnected action exposes the existing retry/reconnect control. In library-graph tests, add narrow-width checks around saved passage evidence, version controls and the purge textarea/confirmation; assert those actions are operable and the page does not overflow. Preserve the existing mutation assertions.
- [x] **Step 3: Run the focused cases and record actual failures before migration.** Reuse current fixtures rather than introducing a new fixture server or mocking away persistence.
- [x] **Step 4: Migrate reader/citations/custom-question styles.** Apply the text styles, bounded measure, fluid padding and clear section rhythm directly to each rendered element. Keep cited opening paragraphs and section content intact. Use the existing analogy/limitations for inset treatment; keep all branch reasons and sources. Style loading, clarification, refresh, errors, disabled states and long URLs. Use local variants for compact/full citations, with keyboard-visible focus that is not clipped.
- [x] **Step 5: Migrate retained topics, connections, versions and library storage.** Use the same surfaces, readable metadata, spacing and shared actions. Preserve disclosures, candidate-vs-saved labels, evidence blocks, pagination, dismissal, read-only states and exact purge confirmation. Use slot recipes only when real coordinated variants warrant them. Remove their old component selectors.
- [x] **Step 6: Rebuild/package and run the focused cases plus all recovery and graph-fixture acceptance.** Expected: long content and confirmations reflow, citations remain valid, old answers remain during refresh, all secondary actions work, and ordinary saved-topic navigation makes no additional research request. Inspect loaded/loading/error/retained readers with keyboard and enlarged text.
- [x] **Step 7: Commit only reader/library migration changes.**

### Task 5: Remove migration scaffolding and verify the complete release

**Files:** Finish `app/src/app.css`, `app/panda.config.ts`, `app/package.json`, `package.json`, `vite.config.ts`, and `.gitignore`; create `app/AGENTS.md`; update `README.md` and this plan's execution evidence. Adjust previously listed tests/configuration only when final checks reveal a concrete issue.

**Interfaces:** Final root CSS declares `reset, vendor, base, tokens, recipes, utilities` and imports vendor CSS into its layer. All application component styles are owned by their component or justified shared recipe; ignored output is regenerated by documented scripts.

- [x] **Step 1: Remove remaining legacy component rules and the temporary legacy layer.** Inspect every old class family and every global selector. Keep document/reset/accessibility defaults in Panda config; retain at most the documented narrow vendor adaptation. Do not replace legacy selectors with a huge global config object. Generated output must not enter Git or lint/format input.
- [x] **Step 2: Audit architecture and emitted output.** Regenerate with the design-system spec; verify semantic-token references, recipe defaults/literal variants, condition coverage, required SvelteFlow classes, and no missing extraction warnings. Inspect built CSS and bundled helpers to document the actual transform coverage. Remove duplicated/dead rules rather than enabling broad `staticCss` to hide missed extraction. Review raw-value escapes and remaining inline library values individually.
- [x] **Step 3: Verify cold-start development and clean checks.** Remove only generated output, run `pnpm check`, and exercise the existing `pnpm dev` supervisor with a disposable private data directory and fixture credentials. Confirm watched style/token changes refresh the frontend and a subsequent check regenerates correct types. Stop only the processes started for this verification.
- [x] **Step 4: Run the final acceptance commands.** `pnpm check`, `pnpm test:unit --run`, `pnpm build`, `pnpm test:build`, and `pnpm test:e2e`. Run `git diff --check`. Expected: all pass, including release assets/CSP and legacy-cookie/HTTPS checks. Audit the updated npm lockfile; record existing upstream findings separately from newly introduced ones without adding exclusions. Backend logic is untouched, so repeat native tests only if a real integration concern justifies it.
- [x] **Step 5: Inspect representative real surfaces with the browser skill.** Capture fixture-only evidence at 320×800, 390×844, 768×1024, 1024×768 and 1440×1000, plus 200% text enlargement/400%-equivalent reflow and reduced motion. Cover start, open workspace, saved library, catalog/retained reader, recovery/error, versions, source evidence and purge confirmation. Check contrast and keyboard focus; use axe where available and inspect issues manually. Record local interaction/layout-shift observations as local measurements, not field Core Web Vitals compliance.
- [x] **Step 6: Document maintainership.** `app/AGENTS.md` points to both official Panda skills and states the real generated import paths, no guessed tokens, semantic roles, local-first recipe decisions, explicit codegen, and no Tailwind/runtime style injection. README documents scripts, ignored artifacts, vendor layering and proven lint/transform coverage. Record known platform/browser gaps honestly.
- [x] **Step 7: Request the required whole-change review, resolve actionable findings, and repeat only affected checks.** Review for the five focus areas and preservation of current app behavior. Commit migration-owned final cleanup/evidence separately from earlier user changes.

## Self-review

Spec coverage: Tasks 1–2 establish Panda, the start experience, shell and saved explorations; Task 3 covers map, navigation, responsive modes and motion; Task 4 covers all readers and library capabilities; Task 5 removes legacy styling and verifies release/accessibility/performance evidence. Every listed component has an owner. All five Review Focus items have explicit verification in their owning tasks. No backend, schema, new content-generation feature, new progress metric, dark theme, font service or mockup-only action is introduced.

Approved for native execution on 2026-10-28. Tasks share one theme, cascade and component vocabulary; final independent review remains required.


## Execution evidence — 2026-10-28

Completed on `codex/trellis-fieldnotes-panda` in the existing checkout. The full migration is present in the working tree; mixed files containing earlier authorized work remain uncommitted as allowed above. Earlier dependency/backend/branding/proxy changes were preserved. No backend or protocol change was introduced by this migration.

- Cold generation: deleted only `app/styled-system/`; `pnpm check` regenerated all helpers/types/spec and passed without extraction, token, lint or Svelte diagnostics. Temporary invalid nesting and token probes failed correctly, then were removed.
- Final checks: `pnpm check`; 48 unit tests; `pnpm build`; seven packaged-release tests; all 29 browser tests pass. `git diff --check` is clean. HTTP/HTTPS, CSP/asset integrity, legacy-cookie upgrade, recovery/cross-tab fencing, view persistence, retained answers, versions, saved connections and purge acceptance are included. Native unit tests were not repeated for this frontend migration; the earlier dependency verification remains separate.
- Development: the full supervisor ran with a disposable private directory, fixture access key and empty provider keys; authenticated frontend/backend flow worked after the initial debug build. Theme-token and colocated Svelte-style edits both refreshed the live browser, and both probes were restored. Own verification processes were stopped.
- Emitted output: root CSS has only the layer order and layered SvelteFlow import. Minification preserves first layers `reset`, `vendor`, `base`, `tokens`, `recipes`, `utilities`; no legacy component rules remain. CSS is approximately 50.8 kB before transfer compression. Static component calls become class strings in the production bundle; necessary dynamic generated recipe helpers remain. No broad `staticCss` or runtime stylesheet injection was added. Rolldown reports an ignored generated pure-annotation placement warning; builds still pass and the warning was not suppressed.
- Visual checks: start at 1440px; workspace at 1440/1024px; reader at 768/390/320px; phone setup/error, saved catalog, retained reader, answer history and purge confirmation; 200% text at 320px and768px; reduced-motion graph behavior. The 320px viewport also represents 400%-equivalent reflow from 1280px. The graph remains a separately navigable two-dimensional viewport.
- Accessibility: axe-core 4.14.0 reports zero violations across the inspected surfaces, including the fixed enlarged drawer. Some scans mark graph/contrast/target-size cases incomplete and required manual inspection; this is not a conformance certification. Measured contrast: body on paper 12.20:1, muted on paper 5.64:1, accent on paper 6.59:1, muted on selected surface 4.83:1, strong control border on paper 3.53:1. Keyboard focus, primary 44px hit areas and inactive-map focus prevention have acceptance coverage.
- Local performance observation: the enlarged drawer click reached two animation frames in 32.1ms on this machine. Recorded shifts came from intentional font/viewport changes and were marked recent input. These observations are not field INP/CLS or Core Web Vitals compliance claims.
- Dependency audit: one existing low-severity `deepmerge` advisory, GHSA-gcqx-w48m-w2h4; no new findings or exclusions. Chromium was exercised locally. Firefox/WebKit, physical phones, hosted CI, external proxy deployment and paid research were not exercised by this migration.

Independent read-only review used a fresh `gpt-6-astra` reviewer. Two findings were fixed in one pass with RED→GREEN regression evidence: the tablet drawer at 200% text (left −288px before; left 32px/right 736px/width 704px after, viewport 768px), and failed nodes relying only on border color (now visible failure text plus an accessible description). The failure regression uses a new uncached question; existing cached answers remain readable after a failed refresh. No deferred minor findings remain.

Rulings carried from execution, in order:

1. Use a feature branch in the existing checkout to preserve approved dirty work/native artifacts. Cost: weaker filesystem isolation than a separate worktree.
2. Leave mixed files uncommitted rather than include earlier edits in migration commits. Cost: commits alone do not contain the full working result; review and handoff must include the working tree.
3. Use scoped Svelte ESLint because the Oxlint bridge misparses these TypeScript Svelte scripts. Cost: extra supported parser/tooling dependencies and an explicit additional check; TypeScript Panda lint remains in Vite+.
4. Make prior content inert while the existing exploration-switch flag is true, preventing stale navigation from racing a pending load. Cost: page controls are temporarily unavailable until loading finishes.
5. Keep earlier dependency/backend/branding/proxy changes outside this migration review, with final release acceptance covering their integration. Cost: earlier defects outside that acceptance are not re-reviewed here.
