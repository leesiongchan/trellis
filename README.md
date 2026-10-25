# Branch

A private knowledge tree. Start with a question, read a detailed explanation, and expand related topics without losing your place. Every opened topic is researched on demand, with links to supporting sources.

## Try the demo without credentials

Choose **Try the microSD demo** on the home screen. It opens eight prewritten topics, including microSD cards → NAND flash → Memory cells → Quantum tunneling, plus controller, error correction, pages/blocks and wear-leveling branches. Expand, collapse, read source links and add your own questions without provider requests.

Demo explorations are labelled in the workspace and saved library. Custom questions stay as unanswered local branches. Every launch creates a separate copy, and reload/resume preserves its questions and view. Research refresh is unavailable in a demo. The app still needs to be reachable to load; this is not an offline-installed app, and following external source links needs connectivity.

## Run locally

Builds use Rust 1.98.0, Node 26.5.0 and pnpm 11.21.0. The packaged runtime is one Rust executable and static assets; it does not need Node. On Nix, retry missing commands through `,`.

```sh
pnpm install --frozen-lockfile --ignore-scripts
pnpm build
cp .env.example .env
```

Set `OPENAI_API_KEY` and `TAVILY_API_KEY` in `.env`. Generate `APP_ACCESS_KEY` with the command shown in `.env.example`. Keep these values local; `.env` is ignored by Git. `OPENAI_MODEL` is configurable and defaults to `gpt-5.6-terra`; availability depends on the API project. A model with Responses structured-output support is required.

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
- The tree, explanations, sources, selected topic and viewport save in IndexedDB in the current browser. Devices and browser profiles do not synchronize. Clearing browser data removes those explorations.
- Conflicting saves from another tab require reloading the saved version. Storage failures keep the current work in memory and show a retry action.
- Topic questions and a bounded ancestor context go to Tavily and OpenAI. API credentials and raw retrieved page text stay on the server; saved explanations retain short evidence quotes and source links.
- Source checks combine exact evidence matching with a separate model-assisted support check. They do not guarantee factual accuracy; original sources remain available to inspect.
- No accounts, analytics, cross-device synchronization or server-side exploration database are included.

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

Browser acceptance runs against the packaged Rust service. Existing UI scenarios mock research responses; additional HTTP/HTTPS scenarios exercise the real Rust request pipeline with local provider fixtures and test cookie upgrades. All routine checks use fake credentials and consume no research credits. Release verification starts the executable from another directory with no Node on its PATH and checks readiness, access, metrics, asset integrity, redaction and SIGTERM exit.

With both providers configured and the built app running, `pnpm live-smoke` makes at most two live research requests. It checks one root and one child, their detailed sections, sources and further branches. Missing credentials cause a clear non-success exit without making requests.

See [the approved design](docs/superpowers/specs/2026-10-25-knowledge-tree-design.md), [implementation plan](docs/superpowers/plans/2026-10-25-knowledge-tree.md) and [research runbook](docs/runbooks/research.md).

## Verification recorded on 2026-10-26

The migration passed 44 Rust tests, 43 frontend/domain/contract tests, four build checks and 21 browser scenarios on macOS. Type checking, strict Clippy, release packaging and dependency audits passed. CI covers Linux and macOS; those hosted jobs have not been run locally.

Live provider research, model availability, a separate physical phone and the Rust release through the actual Tailscale route remain **unverified**. Forwarding was inspected read-only and left unchanged.

See [implementation and review notes](docs/superpowers/2026-10-25-implementation-notes.md) for verification scope and review fixes.
The [offline demo handoff](docs/superpowers/2026-10-25-offline-demo-notes.md) records the demo's behavior, review and verification. See [Rust migration evidence](docs/superpowers/2026-10-26-rust-backend-notes.md) and the [approved Rust design](docs/superpowers/specs/2026-10-25-rust-backend-design.md) for this migration.
