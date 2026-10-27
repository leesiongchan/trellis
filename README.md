# Branch

A private knowledge tree. Start with a question, read a detailed explanation, and expand related topics without losing your place. Questions, immutable answers and citations live in a private embedded SurrealDB library. Saved topics open without another paid research call.

## Try the demo without credentials

Choose **Try the microSD demo** on the home screen. It opens eight prewritten topics, including microSD cards → NAND flash → Memory cells → Quantum tunneling, plus controller, error correction, pages/blocks and wear-leveling branches. Expand, collapse, read source links and add your own questions without provider requests.

Demo explorations are labelled in the workspace and saved library. Custom questions stay as unanswered local branches. Every launch creates a separate copy, and reload/resume preserves its questions and view. Research refresh is unavailable in a demo. The app still needs to be reachable to load; this is not an offline-installed app, and following external source links needs connectivity.

## Run locally

Builds use Rust 1.98.0, Node 26.5.0 and pnpm 11.21.0. The packaged runtime is one Rust executable and static assets; it does not need Node. On Nix, retry missing commands through `,`.

```sh
pnpm install --frozen-lockfile --ignore-scripts
node scripts/fetch-embedding-model.ts .artifacts/embedding-model
pnpm build
cp .env.example .env
```

Set `APP_DATA_DIR` to an absolute private directory outside the repository and release. Optionally set a separate `APP_BACKUP_DIR`; both directories must remain separate from public assets and models. Set `OPENAI_API_KEY` and `TAVILY_API_KEY` in `.env` for new paid research. Generate `APP_ACCESS_KEY` with the command shown in `.env.example`. Keep these values local; `.env` is ignored by Git. `OPENAI_MODEL` is configurable and defaults to `gpt-5.6-terra`; availability depends on the API project. A model with Responses structured-output support is required.

For development after the initial build:

```sh
pnpm dev --port 5173
```

This supervises Vite and Rust on loopback, explicitly using the frontend origin. Open `http://127.0.0.1:5173/access?key=YOUR_APP_ACCESS_KEY` once. The app establishes a private session and removes the key from the address. Without provider keys, the demo and saved explorations remain readable and research reports that setup is required.

For a production build:

```sh
pnpm build
pnpm start
```

Set `APP_ORIGIN=http://127.0.0.1:3000`, `HOST=127.0.0.1` and `PORT=3000` in `.env` for the built server. `pnpm start` launches `release/branch-backend`; it can also run directly without Node. Keep `release/public` and `release/frontend-manifest.json` beside it. Process environment overrides `.env`; set an absolute `APP_ENV_FILE` when launching from another directory. Restart after configuration changes. See the [Rust service runbook](docs/runbooks/rust-service.md) for shutdown, limits, metrics and rollback.

## View privately through Tailscale

Inspect `tailscale serve status` before adding a listener. With HTTPS port 8443 available:

```sh
tailscale serve --bg --https=8443 http://127.0.0.1:3000
```

Set `APP_ORIGIN` to `https://YOUR_MACHINE.YOUR_TAILNET.ts.net:8443`, restart the built app, then open that address with `/access?key=YOUR_APP_ACCESS_KEY` appended on a connected Tailscale device. Preserve the existing scheme, hostname and port on upgrades so IndexedDB explorations remain accessible. The proxy must preserve the external Host header. Use the exact configured origin for research. The key is removed after access. The Mac and server must stay running.

This is private tailnet sharing. Do not enable Funnel. Preserve the existing preview listener on port 60090; the app uses a separate listener. To stop only the app's route:

```sh
tailscale serve --https=8443 off
```

## Behavior and data

- Clicking a topic opens its explanation. The separate expand button reveals children.
- Only opening unexplored topics or explicitly refreshing triggers research. Ready topics are reused.
- Research questions, tree appearances, answer versions and citations save on the server and are available to browsers holding the same access cookie. IndexedDB stores acknowledged offline copies and this device’s selected topic/viewport. Offline research copies are read-only. Demo trees remain local.
- Conflicting revisions require reloading the saved version. Durable operation IDs recover lost responses; research remains server-owned when a tab closes. Interrupted paid work requires an explicit retry.
- Topic questions and a bounded ancestor context go to Tavily and OpenAI. API credentials and raw retrieved page text stay on the server; saved explanations retain short evidence quotes and source links.
- Source checks combine exact evidence matching with a separate model-assisted support check. They do not guarantee factual accuracy; original sources remain available to inspect.
- Similar questions and answer passages connect through reversible graph edges. Local semantic search suggests candidates; accepted connections carry verification provenance. Similarity never merges topics or silently replaces an answer.
- Answers remain pinned per exploration appearance. Refresh advances the chosen appearance; selecting an older saved version is explicit. Deleting an exploration retains its knowledge. Confirmed library purge removes live server content; backups and browser copies require separate clearing.
- Existing browser originals import only after private access and remain untouched. Changed originals become separate labelled copies. Oversized or invalid originals remain available to export. Deleted/purged content cannot silently reappear through automatic import.
- One private shared library is included; there are no accounts or analytics. Holders of the same access key share access.

## Verify

```sh
cargo fmt --manifest-path backend/Cargo.toml -- --check
cargo clippy --manifest-path backend/Cargo.toml --all-targets --all-features --locked -- -D warnings
pnpm test:backend
pnpm check
pnpm test:unit
pnpm build
pnpm test:build
PLAYWRIGHT_SKIP_BROWSER_GC=1 pnpm exec playwright install chromium --only-shell
pnpm test:e2e
pnpm verify:release
, cargo-audit audit --file backend/Cargo.lock
pnpm audit
```

Browser acceptance uses the real Rust/storage pipeline with local provider fixtures, including HTTP/HTTPS cookies, migration, saved navigation and immutable answer history. All routine checks use fake credentials and consume no research credits. Release verification starts the executable from another directory with no Node on its PATH and checks readiness, access, native model degradation, durable imports, exclusive locking, forced restart, backup/restore, fresh epochs, asset integrity, redaction and SIGTERM exit.

With both providers configured and the built app running, `pnpm live-smoke` makes at most two live research requests. It checks one root and one child, their detailed sections, sources and further branches. Missing credentials cause a clear non-success exit without making requests.

See [the approved design](docs/superpowers/specs/2026-10-25-knowledge-tree-design.md), [implementation plan](docs/superpowers/plans/2026-10-25-knowledge-tree.md) and [research runbook](docs/runbooks/research.md).

## Embedded library verification

See [current storage evidence](docs/superpowers/2026-10-27-knowledge-storage-notes.md) and the [storage operations runbook](docs/runbooks/knowledge-storage.md) for measured native gates, platform limits and recovery. Linux/macOS hosted CI, live paid research, a physical phone and deployment through the actual Tailscale route remain unperformed in this workspace. Forwarding and the original checkout are unchanged.
