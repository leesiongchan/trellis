# Trellis: living field guide

Status: approach A selected by the user; this full design is awaiting approval before implementation planning.

Builder check: a fresh frontend reviewer reported “nothing important left” before this design was presented for approval.

## Intent and decisions

Redesign the app with the expressive botanical character of its product page. Trellis should invite another question while remaining comfortable for sustained reading. The user approved the botanical palette, requested related design research, and selected **A / Living field guide** from three interactive workspace studies.

The selected study is `.superpowers/brainstorm/13984-1793281166/content/botanical-app-directions.html`. It demonstrates hierarchy, surfaces, curved connections, question selection and continuation. Its sample content, dimensions, simplified controls and navigation are illustrative; they do not replace the app's data or capabilities.

This is a complete frontend presentation project, extending the existing Svelte 5, SvelteKit, Panda CSS and SvelteFlow implementation. Keep the server-only session model, stored content and research semantics.

## Goals and anti-goals

- Make the start screen, exploration workspace and saved library feel like one inviting product.
- Make the current question, its ancestry and the next available actions easy to recognize.
- Use brief, purposeful responses to selection and expansion to make discovery feel tangible.
- Give real explanations, evidence and follow-up questions generous, readable space.
- Preserve every existing research, recovery, persistence, library and destructive-action capability.

Avoid tiny typography, a generic dashboard, decorative clutter over content, a literal illustrated tree behind live graph labels, perpetual movement, hover-only controls, fake learning progress, and playful language that obscures errors. Do not add quizzes, badges, streaks, account features, generated diagrams or extra provider calls. Backend/API/storage changes, new navigation routes, pane resizing, a new full-map mode and dark mode are out of scope.

## Approaches

**A / Living field guide — selected.** A branching map and generous reader remain adjacent on wide screens. Softly shaped question cards, curved connections, botanical details and short transitions carry the personality. This preserves spatial context during reading. Its tradeoff is less uninterrupted canvas than B.

**B / Discovery table — considered.** An expansive canvas with a floating explanation card prioritizes wandering between questions. The card covers part of the map and provides less stable space for long explanations.

**C / Story trails — considered.** A large editorial reader and compact question outline prioritize sustained reading. The wider branching structure is less apparent. The user chose A over both alternatives.

## Visual language

Keep the palette and semantic roles in `app/src/lib/theme/tokens.ts`: peach page surfaces, warm white reading surfaces, plum text, cobalt actions, lilac canvas/selection, apricot highlights and coral ornaments. Preserve distinct error, warning and success roles. Decoration never carries status information or becomes a low-contrast action color.

Use generous serif headings with readable sans-serif controls and prose. Proposed default: self-host the existing product-page DM Sans and Instrument Serif files, with their license files, under app static assets. The app currently names these families but does not ship their fonts. Keep system fallbacks and `font-display: swap`; no external font request or dependency.

Proposed reading scale: 18px body copy at the default root size, 16px ordinary controls, and at least 14px supporting labels/captions. Use rem-based fluid theme values so enlarged text remains enlarged. Graph titles target 18px and labels 14px at map zoom 1; graph zoom may reduce the rendered size as part of its two-dimensional navigation. Keep reader headings approximately 36–56px and a prose measure around 60–70 characters when space permits. Exact fluid endpoints are implementation calls, not permission to reduce the minimum reading sizes.

Use rounded controls, soft asymmetric corners on a few meaningful surfaces, restrained shadows and botanical leaf/spark motifs. Repeated patterns should look coherent rather than assigning random shapes or colors to topics. Never infer a topic category or research quality from a decorative leaf color.

## Start and return

The question input remains the primary action. Keep the existing example prompts and one clear Explore button. Replace the small-text static diagram with an attractive decorative composition using the already approved product-page canopy artwork. It contains no functional text; prompts and labels remain HTML. Reuse and optimize the existing artwork rather than generating new art. On narrow screens it becomes a smaller accent or is omitted so the input arrives promptly.

Saved explorations appear below the opening as readable continuation cards with existing titles, dates and topic counts. Cards can carry a small abstract botanical mark, but no invented activity or progress metadata. Keep explicit Resume and Delete semantics, delete confirmation, pagination, and the distinction between deleting an exploration and purging retained knowledge.

Maintain the existing separate **Saved topics** and **Your explorations** entry points. The study's simplified “Your library” button does not authorize merging the catalog and exploration drawer. Show real saved/saving/offline/conflict state without squeezing the question area.

## Exploration workspace

Keep a compact header and topic-path toolbar above a near-balanced split, initially the existing map `.9fr` / reader `1.1fr`. The reader scrolls independently; the map remains available. Use the existing `64rem` workspace breakpoint as the starting point and change it only if real-content testing demonstrates a better threshold.

The lilac map uses a quiet dot pattern and light curved connections. Highlight the selected topic clearly with a cobalt boundary and `aria-current`; preserve a separate visible keyboard-focus treatment. A short selected-path response may emphasize the direct incoming edge, using only current selection/ancestry. Do not add visited-state persistence, fabricate a multi-step trail, or recenter the map merely for animation.

Preserve the current layout algorithm, node bounds of **240 × 150 CSS pixels**, depth spacing and saved viewport. Keep select and expand/collapse as separate actions, including SvelteFlow `nodrag`/`nopan` classes and supported styling APIs. Do not scale the whole map to an unreadable overview on entry: preserve current refocus/restoration behavior and keep manual zoom, fit, root, refocus and collapse controls.

Node content priority is relationship/status, a two-line readable question title, then the separate 44px expansion control. If readable typography cannot coexist with the existing teaser, omit the normal teaser rather than shrinking text or changing geometry. Full content remains in the reader and the full question remains the accessible name. Researching and failed states must still have explicit visible text; a status label may replace the relationship line. Preserve the existing failed-state accessible description and a clear route to retry. Do not hide failure behind truncation.

Make root topics warm apricot, ordinary topics warm white, and selected topics visibly outlined. A leaf mark and a small arrow can reinforce the topic-card character. Decoration cannot overlap handles, labels, focus rings or click targets. The prototype's single clickable card does not replace the app's separate selection and expansion actions.

## Reader and continuation

Prioritize relationship, substantial title, opening explanation and cited evidence. Keep paragraph citations adjacent to their text and provide a complete source list; the study's single source chip is not a replacement for the real list. Keep all actual headings, paragraphs, branch suggestions, reasons, citations, analogies and limitations.

Existing analogies receive the apricot inset treatment with an optional small decorative spark. Render it only when an analogy exists. Do not invent an analogy, section title, short answer or rewritten explanation to match the prototype.

Suggested branches become inviting, clearly actionable continuation cards. Each preserves its existing title, reason, evidence and follow behavior. Keep the custom question form and submit label. Use comfortable hit areas and visible focus rather than requiring people to click an unlabeled arrow.

Keep source reading, saved answer history and saved connections in clear sections below the explanation. Retain current native disclosure semantics: history remains open when a newer answer is available, and saved passage evidence remains inspectable. Preserve connection-kind labels, candidate-versus-saved distinction, dismissal rules, refresh, pagination and unavailable/indexing/offline states. Do not add a broad disclosure that hides required error or recovery information.

## Saved and exceptional states

Saved-topic catalog: apply the same card, search-field and typography vocabulary. Preserve search, pagination, close behavior and complete titles. Empty states can use a small botanical accent and concise practical guidance.

Retained-topic reader: keep the explicit read-only banner, saved-version reading and distinct **Start exploration from this topic** action. Starting a new exploration remains different from selecting an answer for an existing appearance.

Research in progress: use the existing truthful status copy, a small finite entrance response and a quiet static placeholder. No percentages or simulated research steps. Refresh keeps the earlier explanation visible until replacement succeeds.

Clarification, failure, disconnected server, save failure, multi-tab conflict and recovery retain their explicit actions and disabled-state rules. Error surfaces use danger roles and direct language; decorative flourishes stay outside alerts. Do not change retry behavior or trigger research on hover, selection, animation or library viewing.

Storage/purge: preserve the exact confirmation phrase, complete explanation of what is deleted, separate backup wording and disabled-state rules. Keep this visually distinct from ordinary navigation and the exploration-delete confirmation.

## Motion and responsive behavior

Use local 150–250ms opacity, boundary or tiny transform responses to acknowledge interaction. New branch nodes can enter with a gentle fade/translation when existing lifecycle behavior allows it; no animation may delay reading, input, focus or research results. Selected-edge emphasis is finite, never a flowing or blinking background. No sound, confetti or perpetual floating foliage.

Honor `prefers-reduced-motion` in CSS and every JavaScript map movement. Existing zoom and fit controls currently need inspection because some use SvelteFlow default durations; reduced-motion support must cover those actions too. Restored viewports and selections must survive refresh, pane switches and navigation.

Below the workspace breakpoint, retain an explicit Map/Read switch tied to the same persisted selected topic. The reader reflows normally; the graph keeps its navigable two-dimensional viewport. Do not introduce the study's simplified stacked question list as a second graph implementation. Preserve existing pane-switch and topic-selection behavior.

Reflow surrounding UI at 320 CSS pixels and with 200% text. Test narrow screens with long titles, long sources/URLs, long confirmation text and open drawers. Aim for WCAG 2.2 AA: text contrast at least 4.5:1 for ordinary text, identifiable controls/focus at least 3:1, visible labels, keyboard operation and roughly 44px primary touch targets. Preserve focus through updates and keep it visible inside scrolling readers/drawers. Decorative graphics are hidden from assistive technology.

## Implementation boundaries and builder calls

Follow `app/AGENTS.md`, official Panda v2 styling/theming guidance and the existing Svelte 5 conventions. Keep strict tokens, both inherited presets, colocated `css`/local recipes and the shared button recipe. Extend shared typography, spacing, shape and motion tokens where repeated use warrants them; component colors remain semantic. Do not edit generated `styled-system` output or introduce a parallel CSS framework.

Scope includes `+page.svelte`, the existing header/start/workspace/map/node/reader components, citations, branch form, exploration list, retained reader, versions, connections and storage UI. Small presentational components may be extracted to keep the page manageable. Add app static fonts/artwork and accompanying licenses; update `docs/design-system.md` with usage intent. Product-page content, README, deployment configuration, dependencies and unrelated dirty files stay outside this work.

Exact decorative silhouettes, icon paths, radii, spacing, fluid type endpoints, motion timing within the stated range and presentational component boundaries belong to the builder. These are proposed defaults, not extra decisions independently stated by the user. Preserve public component contracts unless a small presentational refactor requires a locally coordinated change. No persisted fields or session-layer state should be needed.

## Verification and completion

- Run explicit Panda codegen and inspect resolved token types/spec; run `pnpm check`, frontend unit/integration tests and `pnpm build:frontend`. Report existing warnings separately from new failures.
- Package the frontend with the existing backend and run release asset/CSP checks relevant to added fonts/artwork. Do not rebuild or modify backend behavior merely for styling.
- Run fixture-backed browser acceptance for keyboard, mobile, fieldnotes, tree behavior, persistence/recovery, saved-topic navigation, versions, connections and purge. No paid providers or new research data.
- Inspect start, desktop workspace, phone Map/Read, long reader, library/drawers, retained topics and loading/error states at 320, 390, 768, 1024 and 1440px, plus 200% text and reduced motion. Verify live text is never embedded in artwork and node status/focus are legible.
- Check contrast, accessibility landmarks/headings, complete labels, focus visibility, missing assets, font fallback and layout shifts. Fix presentation/markup accessibility issues encountered in touched surfaces, without adding an unrelated navigation redesign.
- Obtain a fresh implementation review. Commit only authorized frontend/design files. The app has no established hosted deployment here; do not describe a repository push or GitHub Pages product-page update as a deployed application.

These checks establish implementation quality, not a claim of proven best-in-class usability. Choosing and continuing branches without losing context is the design hypothesis; user feedback on the working app remains the practical validation.

## Research rationale

[Explorable Explanations](https://explorabl.es/) offers inspiration for learning through interaction. Applied here: feedback belongs to opening and connecting questions rather than an unrelated reward system. This is a design inference, not evidence that decorative motion improves comprehension.

[NN/G progressive disclosure](https://www.nngroup.com/articles/progressive-disclosure/) supports making frequent actions prominent and secondary detail clearly retrievable. Applied here: preserve visible reading/branching actions and organize evidence/history without burying recovery state.

[Linear's interface redesign](https://linear.app/changelog/2024-03-20-new-linear-ui) describes improving hierarchy, balance, density and clarity. Applied here: consistent navigation and calmer secondary controls support an expressive map and editorial reader; its monochrome aesthetic is not being copied.

The approved product-page design additionally draws on [Hay Festival](https://www.pentagram.com/work/hay-festival?page=1&rel=search), [Natural History Museum](https://www.pentagram.com/work/natural-history-museum) and [Data Humanism](https://giorgialupi.com/data-humanism-my-manifesto-for-a-new-data-wold). Reuse Trellis's existing original artwork and palette rather than copying another identity.
