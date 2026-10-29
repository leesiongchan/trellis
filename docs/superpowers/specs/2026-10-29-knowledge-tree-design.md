# Trellis: an interactive knowledge tree

The user approved replacing the abstract thought knot with an interactive knowledge tree: one starting question at the root, follow-up questions on branches, and a highlighted path when a question is selected. Preserve the existing editorial typography, cream/forest palette, readable 18/16/14px text scale, screenshot section, and source-linked sample notes.

Use a code-native SVG tree so its branches connect to actual question cards. Broad organic stems, small leaves, and restrained orange path highlights make the structure recognizable. Desktop shows an upward-growing tree with the question at its base; narrow layouts put the starting question first and follow a visible stem through three branches. All cards remain semantic buttons with the existing native explanation dialog.

Draw branch endpoints from actual card bounds. Recompute after card content changes, resizing, font loading, and text enlargement. Hover and keyboard focus preview a branch; activation remembers the path until another question is chosen. Changing sample trails resets selection while retaining the existing visited-note behavior. Respect reduced motion and preserve static fallback content.

The approved tree approach makes branching exploration concrete. A literal photographic tree would leave the question links disconnected; another abstract sculpture would repeat the rejected metaphor. This implementation uses vector paths without adding a rendering dependency or bitmap artwork.

Verify path-to-card alignment at 320–1440px and 200% text, all twelve notes, keyboard/focus behavior, screenshot modes, and script failure. Obtain a fresh review and publish through the existing GitHub Pages workflow. Do not read or edit the prior README or unrelated dirty files.

## Verification

- Ninety measured branch endpoints match their actual cards across 320, 390, 768, 1024, and 1440px, all three trails, and normal/doubled root text. No card collisions, reading-text clipping, or horizontal page overflow were observed.
- All twelve sample notes were exercised at both normal and doubled text on the narrowest viewport. Dialog focus, source links, and focus return passed; Enter and Escape were also verified through keyboard input.
- A failing mobile highlight check exposed a stem extending past the selected branch. A separate active-stem path fixes this; the same endpoint check now passes.
- Screenshot crops remain correct at 320px and 390px. Reduced-motion rendering has no transitions. A separate missing-module fixture retains all static questions and the app screenshot, with all ten main sample controls disabled.
- HTML references, font files, image attributes, JavaScript syntax, formatting, and all four existing sample behavior tests pass. The sharing image is 1200×630 and 89,850 bytes, with editable SVG source alongside it.
- Browser accessibility scans reported zero violations. Closed-dialog references and decorative contrast cases flagged for manual inspection were checked separately. Fresh read-only review approved the redesign.

Checks used Chromium; exhaustive cross-browser rendering was outside this bounded change. Live deployment verification is the final publication gate.
