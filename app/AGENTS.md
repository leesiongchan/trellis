# Frontend conventions

Use the official [Panda CSS skill](https://github.com/chakra-ui/panda/blob/v2/skills/panda-css/SKILL.md) and [theming skill](https://github.com/chakra-ui/panda/blob/v2/skills/panda-css-theming/SKILL.md), together with [Thinking in Panda](https://panda-css.com/docs/get-started/thinking-in-panda). Follow the existing Svelte 5 runes and server-only session model.

- Read `panda.config.ts`, `src/lib/theme/` and generated declarations before styling. Never guess a token. Extend both inherited presets; `strictTokens` stays enabled.
- Import generated APIs from `#styled/css`, `#styled/recipes`, `#styled/patterns` or `#styled/tokens`. Component colors use semantic roles; raw palette values belong in primitive definitions. Use `token.var()` for library CSS variables and `token()` when a library needs a resolved value.
- Start with colocated `css()`. Use local `cva()` for a component with genuine visual variants and `sva()` for coordinated slots. Share config recipes only for a real shared API: the button recipe is used across the app. Pass literal variants; do not assemble class names or token names at runtime.
- Define fluid typography and spacing in the theme. Prefer available-width container conditions for component layout. Keep 320px reflow, enlarged text, visible keyboard focus, reduced motion and primary touch targets intact.
- Topic nodes deliberately escape to `[240px]` × `[150px]` to match the graph layout algorithm. Preserve required SvelteFlow `nodrag`/`nopan` classes and public vendor variables. Other raw escapes need a concrete reason.
- `src/app.css` owns only cascade ordering and the SvelteFlow import in `vendor`. Document defaults belong in Panda `globalCss`; application styles stay with their components. Do not introduce Tailwind or runtime style injection.
- `styled-system/` is generated and ignored. Never edit or commit it. Run `pnpm styles:codegen` explicitly after theme/config changes, then inspect its types. `pnpm check` regenerates it; builds also generate through the Vite plugin, so installs with `--ignore-scripts` work.
- `pnpm styles:check` runs Panda extraction with zero tolerated diagnostics and the scoped Svelte ESLint rules. Root Vite+ lints TypeScript with Panda rules. Do not suppress extraction errors or add broad `staticCss` to conceal missing styles.
- Run relevant real-server fixture acceptance for behavior changes. Preserve research, retry/recovery, view persistence, citations, versions, connections and exact purge confirmation; styling must not add paid requests or new data fields.
