# Trellis

A private knowledge tree. Start with a question, read a detailed explanation, and expand related topics without losing your place. Questions, immutable answers and citations live in a private embedded SurrealDB library. Saved topics open without another paid research call.

## Project layout

`app/` contains the Svelte frontend, its configuration, and frontend tests. `api/` contains the Rust service, its tests, and model setup tooling. Shared contract fixtures remain in `tests/fixtures/`; `scripts/` builds, packages, and verifies both parts. Run the commands below from the repository root.

## Run locally

Development tools are pinned by `mise.toml` and `mise.lock`: Rust 1.98.0 with rustfmt/Clippy, Node 26.5.0 and pnpm 11.21.0. Install [mise](https://mise.jdx.dev/getting-started.html), then trust and install this project’s tools. The packaged runtime is one Rust executable and static assets; it does not need Node or mise.

```sh
mise trust
mise install --locked node pnpm rust
mise exec -- pnpm install --frozen-lockfile --ignore-scripts
mise exec -- node api/scripts/fetch-embedding-model.ts .artifacts/embedding-model
mise exec -- pnpm build
cp .env.example .env
```

Set `APP_DATA_DIR` to an absolute private directory outside the repository and release. Optionally set a separate `APP_BACKUP_DIR`; both directories must remain separate from public assets and models. Set `OPENAI_API_KEY` and `TAVILY_API_KEY` in `.env` for new paid research. Generate `APP_ACCESS_KEY` with the command shown in `.env.example`. Keep these values local; `.env` is ignored by Git. `OPENAI_MODEL` is configurable and defaults to `gpt-5.6-terra`; availability depends on the API project. A model with Responses structured-output support is required.

The commands below assume mise is activated in your shell. Otherwise prefix them with `mise exec --`. mise manages the runtimes; Vite+ 1.4.0 is a locked workspace dependency, so a global Vite+ installation is unnecessary.

For development after the initial build:

```sh
pnpm dev --port 5173
```

This supervises Vite+ and Rust on loopback, explicitly using the frontend origin. Open `http://127.0.0.1:5173/access?key=YOUR_APP_ACCESS_KEY` once. The app establishes a private session and removes the key from the address. Without provider keys, saved explorations remain readable and research reports that setup is required.

For a production build:

```sh
pnpm build
pnpm start
```

Set `APP_ORIGIN=http://127.0.0.1:3000`, `HOST=127.0.0.1` and `PORT=3000` in `.env` for the built server. `pnpm start` launches `release/branch-backend`; it can also run directly without Node. Keep `release/public` and `release/frontend-manifest.json` beside it. Process environment overrides `.env`; set an absolute `APP_ENV_FILE` when launching from another directory. Restart after configuration changes. See the [Rust service runbook](docs/runbooks/rust-service.md) for shutdown, limits, metrics and rollback.

## Access through an HTTPS proxy

The server binds to loopback. To access it through an HTTPS proxy, forward to `http://127.0.0.1:3000` and preserve the external Host header. Set `APP_ORIGIN` to the exact browser-facing HTTPS origin, then restart the app. Open that origin with `/access?key=YOUR_APP_ACCESS_KEY` appended to establish a session; the key is removed from the address afterward. Preserve the configured origin and access key on upgrades so existing access links and cookies remain valid.

## Behavior and data

- Clicking a topic opens its explanation. The separate expand button reveals children.
- Only opening unexplored topics or explicitly refreshing triggers research. Ready topics are reused.
- Research questions, tree appearances, answer versions, citations and exploration view state save on the server and are available to browsers holding the same access cookie. The browser keeps no persistent exploration data. A server connection is required to load saved content.
- Conflicting revisions require reloading the saved version. Durable operation IDs recover lost responses; research remains server-owned when a tab closes. Interrupted paid work requires an explicit retry.
- Topic questions and a bounded ancestor context go to Tavily and OpenAI. API credentials and raw retrieved page text stay on the server; saved explanations retain short evidence quotes and source links.
- Pending server operations are discovered after reload. Uncertain requests retain their operation ID for retries within the open tab; requests never admitted by the server are lost on reload. View preferences use the last successful server write across devices.
- Source checks combine exact evidence matching with a separate model-assisted support check. They do not guarantee factual accuracy; original sources remain available to inspect.
- Similar questions and answer passages connect through reversible graph edges. Local semantic search suggests candidates; accepted connections carry verification provenance. Similarity never merges topics or silently replaces an answer.
- Answers remain pinned per exploration appearance. Refresh advances the chosen appearance; selecting an older saved version is explicit. Deleting an exploration retains its knowledge. Confirmed library purge removes live server content; retained backups require separate clearing.
- Demos and automatic browser imports have been removed. Existing server-imported knowledge remains available. Previous browser databases are left untouched but are no longer read or imported by the app.
- One private shared library is included; there are no accounts or analytics. Holders of the same access key share access.

## Frontend tooling

Vite+ runs development, production builds and unit tests from `app/vite.config.ts`. Root `vite.config.ts` holds workspace lint and formatting policy. `pnpm check` runs Svelte diagnostics plus Vite+ format/lint/type checks; `pnpm fmt` formats supported project files. Rust, vendored code, canonical fixtures and historical design records keep their existing formatting.

Panda CSS owns the Fieldnotes theme. `app/panda.config.ts` composes semantic tokens, fluid text styles and the shared button recipe; components colocate `css`, `cva` and `sva` styles. See [frontend conventions](app/AGENTS.md) and the [approved design](docs/superpowers/specs/2026-10-28-trellis-fieldnotes-panda-design.md).

`pnpm --dir app styles:codegen` generates helpers, declarations and the design-system spec in ignored `app/styled-system/`. `pnpm check` runs it explicitly before Svelte checking, then `styles:check` checks extraction and Svelte token usage. Vite+ applies Panda rules to TypeScript; scoped ESLint uses the supported Svelte/TypeScript parsers because the Oxlint bridge cannot parse these Svelte scripts correctly. No install lifecycle hook is required. Generated files are excluded from formatting and linting.

The Panda Vite plugin generates CSS and watches source/theme changes. Source transforms are enabled; static calls become classes where supported, while dynamic recipes retain generated helpers. `app/src/app.css` declares `reset, vendor, base, tokens, recipes, utilities` and imports SvelteFlow into `vendor`, ahead of application recipes/utilities. No legacy component stylesheet or runtime stylesheet injection remains.

Dependency manifests use caret ranges; pnpm and Cargo lockfiles record the tested versions. TypeScript 7 is installed as `@typescript/native` for its native `tsc` executable. The `typescript` alias points to the official `@typescript/typescript6` compatibility package because SvelteKit and `svelte-check` still require the JavaScript compiler API. Both follow the [TypeScript team's side-by-side setup](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/#running-side-by-side-with-typescript-6-0).

Use `pnpm dev` (or `pnpm exec vp run dev`) to supervise both Vite+ and Rust. From `app/`, built-in `vp dev` starts only the frontend and `vp build` builds only the frontend. The root `pnpm build` (or `pnpm exec vp run build`) also builds Rust and packages the complete release. Playwright, Cargo and native release acceptance remain separate checks. No Vite Task caching is enabled for research, browser or release checks.

## Verify

Install the optional audit CLI once with `mise exec -- cargo install cargo-audit --version 0.22.2 --locked` before running `cargo audit` below. CI installs the same pinned version.

```sh
cargo fmt --manifest-path api/Cargo.toml -- --check
cargo clippy --manifest-path api/Cargo.toml --all-targets --all-features --locked -- -D warnings
pnpm test:backend
pnpm check
pnpm test:unit
pnpm build
pnpm test:build
PLAYWRIGHT_SKIP_BROWSER_GC=1 pnpm --dir app exec playwright install chromium --only-shell
pnpm test:e2e
cargo audit --file api/Cargo.lock
pnpm audit
```

Browser acceptance uses the real Rust/storage pipeline with local provider fixtures, including HTTP/HTTPS cookies, server view persistence, recovery, saved navigation and immutable answer history. All routine checks use fake credentials and consume no research credits. `pnpm test:build` includes release verification; use `pnpm verify:release` to run that check alone. Browser acceptance uses the single `app/playwright.config.ts` and owns isolated native test servers, never the normal development database. Release verification starts the executable from another directory with no Node on its PATH and checks readiness, access, native model degradation, durable imports, exclusive locking, forced restart, backup/restore, fresh epochs, asset integrity, redaction and SIGTERM exit.

With both providers configured and the built app running, `pnpm live-smoke` makes at most two live research requests. It checks one root and one child, their detailed sections, sources and further branches. Missing credentials cause a clear non-success exit without making requests.

See [the approved design](docs/superpowers/specs/2026-10-25-knowledge-tree-design.md), [implementation plan](docs/superpowers/plans/2026-10-25-knowledge-tree.md) and [research runbook](docs/runbooks/research.md).

## Embedded library verification

See [current storage evidence](docs/superpowers/2026-10-27-knowledge-storage-notes.md) and the [storage operations runbook](docs/runbooks/knowledge-storage.md) for measured native gates, platform limits and recovery. Linux/macOS hosted CI, live paid research, a physical phone and deployment through an external HTTPS proxy remain unperformed in this workspace.
