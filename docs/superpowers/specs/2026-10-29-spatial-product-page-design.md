# Trellis: a spatial poster

The first product page was too conventional: repeated two-column sections, botanical decoration, miniature labels, and a familiar hero/demo/features/FAQ sequence. The user requested a more creative and modern redesign. Replace its composition and interaction, rather than merely changing its surface styling.

## Research and decisions

- [Sublime](https://sublime.app/), inspected in a desktop browser: strong typographic scale, tactile imagery, and a clear central visual idea. Borrow the confidence and material contrast; create original art and composition.
- [Muse research, Ink & Switch](https://www.inkandswitch.com/muse/): a spatial workspace can make connections tangible. Let the opening itself be a constellation of selectable questions.
- [Potluck, Ink & Switch](https://www.inkandswitch.com/potluck/): embed useful interaction directly in the content. The demonstration belongs in the opening, rather than a separate generic feature panel.
- [Bret Victor, Explorable Explanations](https://worrydream.com/ExplorableExplanations/): keep the reader's own investigation visible and make the route from question to explanation direct. This page uses authored sample notes, not a computational model or live research.
- [Cosmos](https://www.cosmos.so/): treat collected ideas as a visual environment, using changes in scale and proximity to suggest discovery.

## Direction

An editorial spatial poster: oversized grotesk typography with an Instrument Serif counterpoint; forest, cream, and a restrained citron accent; an original dimensional thought-knot sculpture; freeform question cards connected with actual graph lines. A second, dark forest composition enlarges the app screenshot and lets visitors inspect the map or reader. Finish with a compact self-hosting invitation and an oversized wordmark.

The three alternatives considered were a refined botanical journal (too close to the rejected direction), an entirely dark technical canvas (too far from the app's warmth), and this spatial poster. Choose the latter: preserve brand colors and typography while changing hierarchy, visual rhythm, imagery, and page structure.

## Implementation

- Replace `site/index.html`, `site/styles.css`, and `site/main.js`.
- Retain the source-linked sample data and four behavior tests in `site/trail.js` and `tests/site/trail.test.mjs`.
- Use semantic buttons for question selection and a native dialog for the selected sample explanation. Preserve focus on close and support Escape.
- Add screenshot inspection controls for whole workspace, map, and reader. Their pressed states and visual crop must agree.
- Generate an original transparent thought-knot asset with the built-in image tool, save it under `site/assets/`, and record its prompt alongside these notes.
- Keep the entire page static and keep all fonts and assets local. Never imply the illustration is a product feature or the sample is live research.
- Keep 320px reflow, keyboard access, visible focus, reduced motion, and readable script-failure content. No scroll hijacking, custom pointer, or endless auto-animation.
- Verify the new composition in desktop and phone screenshots, test actual interactions, obtain a fresh review, and deploy through the existing Pages workflow.

## Original artwork

Built-in image generation, not the API/CLI fallback. Final workspace asset: `site/assets/thought-knot.webp`.

Final prompt:

> Use case: stylized-concept. Asset type: original editorial 3D artwork for Trellis, an app about branching questions and connected understanding. Create a single sculptural 'thought knot', an impossible but physically convincing intertwining of three broad continuous ribbons: one matte very dark forest-green painted metal, one warm ivory layered paper with visibly fine book-page edges, one translucent pale citron glass. The ribbons loop in a branching, open, asymmetric trefoil-like structure with three large negative-space openings. Two tiny burnt-orange polished spheres and a few hairline silver connecting rods create a subtle sense of a diagram becoming an object. Premium contemporary design-studio still life, tactile PBR materials, macro-quality detail, soft directional studio lighting with sculptural shadows and ambient occlusion. Three-quarter view, entire sculpture visible, centered, filling about 85% of square frame, generous clean silhouette and especially clear holes. Genuine transparent background, isolated cutout with natural soft contact shadow only, no floor or rectangular backdrop. Not a logo, not a flat icon, not botanical. No leaves, flowers, moss, stone, plants, books, screens, text, letters, labels, watermark, glow, neon, gradients in background, or decorative particles. Object should feel strange, intelligent, collectible, and carefully art-directed rather than like a generic AI illustration.


The generated transparent PNG was optimized to WebP (249,760 bytes) for delivery; the original generated PNG remains in the image-generation output directory.


## Verification

- Four sample-data behavior tests pass.
- All twelve root/branch notes open with matching sources; closing or pressing Escape returns focus to the triggering card.
- All three screenshot modes preserve the correct full source columns at 320px and 390px, with measured framing error below 0.001px. This fixes the material issue found in fresh review.
- At 320px with 200% text size, page width remains 320px and the constellation flows vertically without clipped titles.
- A separate missing-script fixture leaves all ten main demo controls disabled and keeps the headline and app screenshot readable.
- HTML has no duplicate IDs or attributes; local assets and anchors resolve; formatter and JavaScript syntax checks pass.
- The complete public artifact is approximately 1.18 MB, including fonts, the app screenshot, optimized artwork, and sharing image.
- A fresh browser accessibility scan reported zero axe violations.
