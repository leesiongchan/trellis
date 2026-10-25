# Branch

A private knowledge tree. Start with a question, read a detailed explanation, and expand related topics without losing your place. Every opened topic is researched on demand, with links to supporting sources.

## Try the demo without credentials

Choose **Try the microSD demo** on the home screen. It opens eight prewritten topics, including microSD cards → NAND flash → Memory cells → Quantum tunneling, plus controller, error correction, pages/blocks and wear-leveling branches. Expand, collapse, read source links and add your own questions without provider requests.

Demo explorations are labelled in the workspace and saved library. Custom questions stay as unanswered local branches. Every launch creates a separate copy, and reload/resume preserves its questions and view. Research refresh is unavailable in a demo. The app still needs to be reachable to load; this is not an offline-installed app, and following external source links needs connectivity.

## Run locally

Requires Node 24+ and pnpm 11.21.0. On a Nix system, missing commands can be run through `,`.

```sh
pnpm install --frozen-lockfile
cp .env.example .env
```

Set `OPENAI_API_KEY` and `TAVILY_API_KEY` in `.env`. Generate `APP_ACCESS_KEY` with the command shown in `.env.example`. Keep these values local; `.env` is ignored by Git. `OPENAI_MODEL` is configurable and defaults to `gpt-5.6-terra`; availability depends on the API project. A model with Responses structured-output support is required.

For development, set `APP_ORIGIN=http://127.0.0.1:5173`, then:

```sh
pnpm dev --port 5173
```

Open `http://127.0.0.1:5173/access?key=YOUR_APP_ACCESS_KEY` once. The app establishes a private session and removes the key from the address. Without provider keys, saved explorations remain readable and research reports that setup is required. No sample content is substituted for live research.

For a production build:

```sh
pnpm build
pnpm start
```

Set `HOST=127.0.0.1` and `PORT=3000` in `.env` for the built server. `pnpm start` loads `.env`; restart the process after changing credentials or the origin.

## View privately through Tailscale

Inspect `tailscale serve status` before adding a listener. With HTTPS port 8443 available:

```sh
tailscale serve --bg --https=8443 http://127.0.0.1:3000
```

Set `APP_ORIGIN` and `ORIGIN` to `https://YOUR_MACHINE.YOUR_TAILNET.ts.net:8443`, restart the built app, then open that address with `/access?key=YOUR_APP_ACCESS_KEY` appended on a connected Tailscale device. Use the exact configured origin for research. The key is removed after access. The Mac and server must stay running.

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
pnpm check
pnpm test:unit
PLAYWRIGHT_SKIP_BROWSER_GC=1 pnpm exec playwright install chromium --only-shell
pnpm test:e2e
pnpm build
pnpm audit
```

Browser acceptance tests mock only the research endpoint; they verify branching, citations, reloads, phone navigation, storage failures, interrupted/out-of-order work and two-tab conflicts. They do not consume research credits.

With both providers configured and the built app running, `pnpm live-smoke` makes at most two live research requests. It checks one root and one child, their detailed sections, sources and further branches. Missing credentials cause a clear non-success exit without making requests.

See [the approved design](docs/superpowers/specs/2026-10-25-knowledge-tree-design.md), [implementation plan](docs/superpowers/plans/2026-10-25-knowledge-tree.md) and [research runbook](docs/runbooks/research.md).

## Verification recorded on 2026-10-25

The local implementation passed 35 unit/integration tests and 17 browser scenarios, including the provider-free demo, isolation between demo copies, phone layout, reloads, switching saved viewports, conflicting saves, failed refreshes, unreadable saved records and deletion during research across tabs. Type checking and the production build passed. The dependency audit reported no known vulnerabilities.

Live provider-backed research is **unverified** and excluded from this handoff at the user's request. Local browser tests through a Tailscale URL verify that endpoint, not a separate physical device.

See [implementation and review notes](docs/superpowers/2026-10-25-implementation-notes.md) for verification scope and review fixes.
The [offline demo handoff](docs/superpowers/2026-10-25-offline-demo-notes.md) records the demo's behavior, review and verification.
