# Trellis product page: readable type

The user likes the spatial poster direction but finds its small text difficult to read. Keep the artwork, colors, large headings, and sample interactions. Improve the reading scale and give the content room to wrap.

## Research applied

- [Nielsen Norman Group: Legibility, Readability, and Comprehension](https://www.nngroup.com/articles/legibility-readability-comprehension/): use a reasonably large default font, strong contrast, and user-controlled enlargement. Visual hierarchy should not depend on text becoming difficult to see.
- [WCAG 2.2: Resize Text](https://www.w3.org/WAI/WCAG22/Understanding/resize-text.html): text must support enlargement to 200% without losing content or functionality. This does not prescribe a minimum font size.
- [WCAG 2.2: Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html): ordinary content should fit a 320 CSS pixel viewport without horizontal reading scroll.
- [WCAG 2.2: Target Size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html): the AA baseline is 24 CSS pixels with exceptions. Keep this page's main controls at least 44 pixels high for a more forgiving touch target.

## Implementation choices

Use three local, relative-unit tokens: 18px body copy, 16px controls, and 14px supporting labels at the browser's default 16px root size. Retain these sizes on phones. These values are design choices informed by the research, not a claim that WCAG mandates them.

Widen desktop question cards, increase line height, and remove the shrinking breakpoint overrides. Let phone cards form a vertical, staggered composition with the sculpture between the starting question and its branches. Use a container query to let larger-screen cards flow when enlarged text needs more room. Wrap screenshot controls on phones while preserving the three existing screenshot crops.

Verify rendered type sizes, overflow at 320–1440px, 200% text enlargement, keyboard/dialog interactions, source links, screenshot modes, and browser accessibility checks. Publish through the existing GitHub Pages workflow after review.

## Verification

- At 320, 390, 768, 1024, and 1440px, both default and doubled root text size show no horizontal page overflow or clipped reading elements. Main visible controls are at least 44px high. Reading labels render at least 14px by default.
- All twelve sample notes fit the 320px dialog at doubled text size. Close buttons remain inside the frame, and closing returns focus to the triggering card. Enter opens the focused card; Escape closes it.
- Screenshot map and reader crops retain their framing at 320px and 390px.
- The four sample behavior tests, JavaScript syntax check, formatter, HTML references, and image/anchor checks pass. A browser axe scan reports zero violations; closed-dialog ARIA references and decorative contrast checks require manual inspection and were checked separately.
- Fresh read-only review approved the changes after inspecting desktop, intermediate, and narrow layouts and independently verifying the enlarged dialog.

Browser checks used Chromium. Cross-engine rendering and native browser zoom behavior were not separately verified.
