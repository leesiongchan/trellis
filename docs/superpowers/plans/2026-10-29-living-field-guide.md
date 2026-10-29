# Living Field Guide Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` to implement these tasks in this session, with one fresh review at the end. Track completed tasks in the plan ledger.

**Goal:** Apply the approved botanical living field guide across the real app without changing research or library semantics.

**Architecture:** Extend existing Panda tokens and Svelte components. Keep SvelteFlow geometry, session contracts and separate map/reader navigation; add only presentational assets and local interaction feedback.

**Tech Stack:** Svelte 5, SvelteKit, Panda CSS v2, SvelteFlow, Playwright, Vite+.

**Spec:** `docs/superpowers/specs/2026-10-29-living-field-guide-design.md` (approved).

## Global Constraints

- Preserve server-only state, research calls, recovery, citations, versions, connections and exact purge confirmation.
- Preserve 240 × 150 CSS-pixel node geometry and the existing layout/restoration behavior.
- Keep both Panda presets, strict tokens and zero extraction diagnostics; generated output stays ignored.
- Ordinary controls are 16px, supporting copy at least 14px, body copy at least 18px at the default root size.
- Reflow surrounding UI at 320px and 200% text; respect reduced motion in CSS and every map control.
- Reuse existing artwork/fonts/licenses; no new dependencies, backend changes or paid research.
- Do not read/change README or unrelated dirty files; product-page content and deployment configuration are out of scope.

## Review Focus

- Long titles and failure labels must fit map nodes without losing the separate expand action: inspect rendered bounds and run fieldnotes/mobile acceptance.
- Enlarged text and open drawers must retain controls: run the existing 320px/200% and tablet drawer scenarios.
- Refresh, clarification and failure must preserve content and recovery actions: run research/recovery browser acceptance.
- Saved-topic/version/connection navigation must preserve existing semantics: run library-graph/persistence acceptance.
- Reduced-motion zoom/fit/refocus must settle immediately without discarding viewport state: add a focused map-control browser assertion.

## Task 1: Theme, assets and the welcoming start screen

**Files:** `app/src/lib/theme/tokens.ts`, `button.recipe.ts`, `app/panda.config.ts`, `app/src/app.html`, `app/src/routes/+page.svelte`, `TopicStart.svelte`, `AppHeader.svelte`; new `BotanicalMark.svelte`, `StartIllustration.svelte`; `app/static/fonts/`, `app/static/images/`.

**Interfaces:** Preserve existing component props. `BotanicalMark` exposes a small literal `kind` variant (`leaf` or `spark`) for decorative SVG; `StartIllustration` is decorative with no interaction/state. Existing semantic palette roles remain stable.

- [x] Extend fluid type/shape tokens; use 18px minimum body, 16px normal controls, 14px captions and larger serif reader headings. Make buttons pill-shaped with clear boundaries/focus.
- [x] Self-host the existing licensed fonts through Panda global `@font-face` rules and app preload links. Copy the approved canopy to app static images; keep it decorative and out of the phone's main input flow.
- [x] Replace the small SVG diagram with `StartIllustration`; preserve examples, input/submit behavior and resume actions. Refine header hierarchy while retaining both saved entry points and real save states.
- [x] Run `pnpm --dir app styles:codegen`, inspect generated types/spec, then `pnpm check` and `pnpm test:unit`. Expected: zero new errors/extraction warnings and 45 existing unit tests pass.
- [x] Visually verify start/header at 320, 390 and 1440px. These reversible style changes use visual checks and existing tests, not tests that mirror CSS literals.

## Task 2: Readable botanical map and responsive workspace

**Files:** `TopicNode.svelte`, `TreeCanvas.svelte`, `Workspace.svelte`, `WorkspaceControls.svelte`; new `app/tests/e2e/living-field-guide.spec.ts`.

**Interfaces:** Preserve node callback props, graph layout, node size and session setters. Use SvelteFlow supported edge style/type APIs for curves and selected incoming-edge emphasis. Use explicit zero durations when reduced motion applies.

- [x] Add a focused fixture browser scenario that checks 18px topic titles, 14px labels, separate 44px expand controls, explicit failed-state text and immediate reduced-motion zoom/fit settlement; run against baseline and confirm the typography/landmark assertions fail for the expected reason.
- [x] Restyle fixed-size nodes with a readable title/relationship, decorative leaf, apricot root and clear selected/focus states; omit the ordinary teaser where needed, preserving full accessible question names and explicit failure description.
- [x] Replace angular smoothstep edges with supported curved edges; derive incoming selected-edge color without adding persisted state or perpetual motion. Keep restoration and refocus logic.
- [x] Refine map controls, path toolbar and Map/Read switch. Add a workspace main landmark and correct map-controls grouping without changing navigation behavior.
- [x] Run the new scenario plus `fieldnotes.spec.ts`, `mobile.spec.ts`, `knowledge-tree.spec.ts` against a newly packaged frontend. Expected: all pass; inspect desktop, 200% text and phone map/reader bounds.

## Task 3: Editorial reader and inviting continuation/library surfaces

**Files:** `TopicReader.svelte`, `BranchQuestion.svelte`, `CitationLink.svelte`, `ExplorationList.svelte`, `AnswerVersions.svelte`, `RetainedTopic.svelte`, `TopicConnections.svelte`, `LibraryStatus.svelte`, catalog/drawer presentation in `+page.svelte`.

**Interfaces:** Preserve every existing prop, callback and evidence mapping; presentational components may be extracted without changing session contracts. Use Task 1 type/shape tokens and `BotanicalMark`.

- [x] Give the reader an h1 topic title, bounded prose, a botanical relationship line, conditional apricot analogy and larger, wrapping source links. Preserve complete explanation/source data and honest refresh/loading/error/clarification states.
- [x] Restyle all branch suggestions as inviting cards while retaining existing accessible names, reasons and evidence. Refine the labeled custom-question form with an explicit submit target.
- [x] Apply continuation-card styling to saved explorations and catalog topics; refine drawer headings, empty states and retained-topic banner. Preserve all delete confirmations and exact library purge behavior.
- [x] Refine version, connection and storage sections with consistent hierarchy and existing native disclosures; never hide newer-answer/recovery state or remove pagination/evidence.
- [x] Run fixture acceptance for fieldnotes/mobile, research/recovery, persistence, library graph, versions/connections and purge. Expected: no changed behavior, no extra research calls and no loss of retained-topic read-only semantics.

## Task 4: Integration, visual review and delivery

**Files:** `docs/design-system.md`, this plan and approved spec; narrowly scoped fixes to the surfaces above.

**Interfaces:** Full app uses one semantic design system. Static assets must pass existing release allowlist/CSP/manifest checks; no external font or image services.

- [x] Run `pnpm check`, `pnpm test:unit`, `pnpm build:frontend`, package the frontend using the existing Rust executable, run `pnpm test:build` and fixture browser acceptance. Expected: tests pass, no new check failures; record existing warnings separately.
- [x] Inspect start/workspace/library/retained/error/loading states at 320, 390, 768, 1024 and 1440px, 200% text and reduced motion. Check contrast, labels, landmarks, keyboard focus, complete error text, font fallback and missing assets.
- [x] Update design-system usage guidance and mark the spec approved. Request one fresh whole-change review, address important findings, and rerun only affected checks.
- [x] Commit scoped changes, integrate into the original checkout while preserving unrelated work, and push using the session's existing authorization. Report repository delivery accurately; no application deployment is configured.

## Execution decision

Native execution in this session, with one independent final review. This is the assistant's implementation choice: tasks share the same theme/components, and the user has approved implementing the complete design. No additional plan-approval round is necessary for choices already covered by that approval.

## Verification evidence

- Complete fixture browser suite: 32 passed.
- Frontend unit suite: 45 passed.
- Release/build checks: 5 passed.
- Svelte/Panda diagnostics: zero errors and warnings; four pre-existing root lint warnings in site tests.
- Visual and keyboard/reflow review completed at the specified widths, including 200% text and reduced motion.
- Independent final review approved the implementation and responsive fixes.

Execution ruling: shared theme/component work was validated and reviewed as one integrated change, avoiding repeated full-suite runs for each presentation-only task.
