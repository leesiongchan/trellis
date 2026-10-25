# Rust backend migration

Status: draft for user approval. Approach 1 was selected by the user on 2026-10-25; implementation is not yet approved.

Builder check: a fresh Rust backend reviewer read this document on 2026-10-25 and reported “nothing important left.” This is a design-completeness check, not implementation verification.

## Intent and scope

Replace Branch's Node backend with production-quality, idiomatic Rust. Run one Rust process that serves both the existing Svelte frontend and the private research API. Preserve the knowledge-tree experience, browser-local saved explorations, and provider-free demo.

The user explicitly requires Rust, production readiness, and modern best practices and selected a Rust service serving the API and built frontend. The assistant proposes retaining the existing private, single-user, loopback-and-Tailscale operating model. The user's earlier “ok” did not specify a new hosting or account model; approving this design accepts that scope.

This is a backend migration with frontend build and operational changes, large enough to require a full implementation plan. Production quality here means bounded resource use, secure boundaries, predictable failures, diagnosable operation, reproducible builds, verified behavior, and a documented upgrade/rollback path for one service.

## Current system

- SvelteKit 3 with Svelte 5 and adapter-node. Configuration lives in `vite.config.ts`.
- `GET /access?key=...` establishes a seven-day signed `branch_access` cookie and redirects to `/`.
- `POST /api/research` validates access, origin, input, and provider configuration; coordinates limited, deduplicated research jobs; and returns validated structured results.
- Research is Tavily retrieval, OpenAI generation, deterministic evidence checks, then a separate model-assisted grounding check. Insufficient evidence and clarification are valid results.
- The browser owns IndexedDB persistence, tree state, viewport state, demos, and conflicts between tabs. The homepage has no server-side loader; browser storage is accessed after mounting.
- The production service binds to loopback and is privately forwarded with Tailscale Serve. The existing preview listener on port 60090 is separate infrastructure.
- Provider-backed research has not been verified live. Prior documentation explicitly excluded paid live verification at the user's request.

## Goals, non-goals, and anti-goals

Goals:

- Replace all application server behavior with Rust while retaining the Svelte UI.
- Preserve existing request/result contracts, browser storage, demo isolation, access links, and research protections.
- Make timeouts, admission limits, cancellation, shutdown, and provider failures explicit and testable.
- Produce a documented release artifact and automated checks that exercise the actual Rust server.

Non-goals:

- Accounts, public hosting, multiple users, collaboration, server-side exploration storage, synchronization, export/import, or a database.
- Queued durable jobs, restart replay, distributed rate limiting, autoscaling, or a multi-process backend.
- New providers, new models, changed explanation behavior, streaming output, a frontend rewrite, or a PWA.
- Deploying this change, changing active Tailscale configuration, or making paid provider calls as part of design or routine verification.

Anti-goals:

- A nominal Rust API that still requires Node to run the production app.
- Losing access to saved explorations because the public scheme, hostname, or port changed.
- New unbounded queues, hidden provider retries, leaked secrets, or errors that bypass the documented API envelope.
- Claiming that mocked provider tests prove live research quality or that browser-local storage is a backup.

## Approaches

### 1. Rust serves the API and static Svelte frontend — selected

Build the SvelteKit frontend into static files. One Rust service handles access, research, health, operational metrics, and static assets on the same origin. The user selected this approach after the assistant recommended it for one-process operation and same-origin requests.

Tradeoff: the release artifact includes a frontend build and a Rust executable, and frontend build/security-header behavior needs explicit integration testing. Node and pnpm are still build/test tools.

### 2. Rust API alongside the SvelteKit Node server — not selected

Retain Node for server rendering and proxy API requests to Rust. This reduces frontend build changes and retains runtime SSR, but adds a production process and proxy boundary. The current app has no user-data SSR requirement that justifies that operational cost.

## Architecture and idiomatic Rust

Assistant design choices, subject to this document's approval:

- One Cargo package under `backend/`, with a reusable library for application composition and a thin executable for startup, signals, and shutdown. Avoid a workspace of speculative microcrates.
- Rust 2024 edition on a pinned stable toolchain compatible with the selected dependencies. Commit `Cargo.lock`; select exact compatible toolchain/dependency versions during planning after checking current official documentation.
- Axum and Tokio for HTTP and async execution; Tower/Tower HTTP for appropriate middleware and static serving.
- Serde for explicit wire types, Reqwest with Rustls for pooled outbound HTTPS, `thiserror` for typed application/provider failures, and contextual startup errors at the executable boundary.
- `tracing` and JSON log output; in-process metrics with a private Prometheus-format endpoint. No mandatory hosted monitoring service or separate collector process.
- Separate modules for configuration, HTTP/access, contracts/validation, job coordination, research orchestration, provider adapters, grounding, static delivery, and telemetry.
- Use typed enums for research outcomes and failure codes. Keep upstream response models separate from validated application models. Use traits only where they support provider substitution and meaningful tests; avoid speculative generic frameworks.
- Own configuration and reusable clients in application state. Share state through `Arc` where necessary, borrow data where ownership is unnecessary, keep critical sections short, and never hold coordination locks across network awaits.
- No application-authored unsafe code, production `unwrap`/`expect`, silent error swallowing, blocking network I/O on async workers, or secrets in `Debug` output. Use RAII guards for permits and task cleanup.

## Frontend build, static delivery, and development

Switch SvelteKit to adapter-static and prerender `/` as an app shell. Existing browser lifecycle guards and IndexedDB behavior remain intact. Runtime SSR is removed; prerendering is a build step. Move the two server routes and server security hooks into Rust, then remove obsolete Node server modules and dependencies when their Rust replacements are verified.

The release is a Rust executable plus a dedicated, read-only frontend asset directory and build metadata, packaged together. The default asset location is resolved relative to the executable; an explicit asset-directory setting supports development/tests. A missing or invalid frontend build is a startup error in production. Never serve the repository root, `.env`, source files, or arbitrary symlink targets outside the release asset root.

Route API/access/operational paths before static files. Only `/` requires HTML; do not add a catch-all SPA fallback for unknown API or asset paths. Return 404 for missing assets and unknown paths, and 405 for unsupported methods. Do not expose directory listings. Hashed immutable assets receive long-lived caching; HTML and mutable build metadata require revalidation; access, research, and operational responses use `no-store`.

Preserve the current CSP intent and other security headers in Rust, including API errors and redirects. Prerendered hydration scripts must be authorized with exact build-generated hashes or emitted without inline code; never add script `unsafe-inline` or `unsafe-eval`. Incorporate any generated script hashes into the HTTP CSP. Retain the existing inline-style allowance needed by the UI. Verify the production browser has no CSP violations; meta-only CSP does not substitute for HTTP `frame-ancestors`. Apply HSTS only on the configured HTTPS deployment.

Development keeps Vite hot reload. Vite on loopback port 5173 proxies `/access`, `/api`, `/health`, and `/metrics` to Rust on a separate loopback port, preserving the browser-facing Host and Origin. No permissive CORS middleware is needed. Development scripts and environment examples make both processes and their ports explicit. Only the Rust process runs in production. Existing command names such as `pnpm build`, `pnpm start`, and verification commands should remain useful through updated scripts; their exact composition is left to planning.

## API and data compatibility

Retain `POST /api/research` and its existing camelCase request fields: `explorationId`, `nodeId`, `question`, nullable `clarification`, and `ancestors` containing title/summary pairs.

Retain the three discriminated success shapes: `topic` with content, `clarification` with a question, and `insufficient-evidence` with a message. Retain the error shape `{ error: { code, message, retryable, requestId } }`, existing error codes and meaningful HTTP statuses. New operational failures may add documented codes without altering the shape. Framework rejections, body limits, deadlines, wrong methods, and unknown `/api/*` paths must use the same envelope. Errors contain safe messages and no raw upstream bodies or internal stack traces.

Rust validates all existing structural and semantic bounds from `src/lib/knowledge/schemas.ts`, including trimmed questions, IDs, nullable/required fields, array limits, total ancestor summaries, public HTTPS source links, evidence lengths, section/branch counts, and timestamps. Preserve Zod's unknown-field stripping behavior at this boundary. Match JavaScript UTF-16 length semantics where the existing schema uses string length; UTF-8 bytes remain the independent transport limit. Share JSON contract fixtures between Rust and TypeScript and check both sides against valid and invalid cases, including non-ASCII boundaries. Do not change persistence schema version 1 or its default `research` mode.

Existing TypeScript schemas remain the browser validator; Rust is the authoritative server validator. Avoid a new code-generation stack unless planning identifies a concrete need. Contract fixtures must detect drift in both directions.

## Security boundaries and private access

Assets to protect are provider credentials, access capabilities, private questions/context, saved browser data, and paid provider capacity. Untrusted boundaries are browser requests, headers/cookies, retrieved source text, provider/model responses, and static-file paths.

- Preserve the access-link flow, cookie name, seven-day expiration, and Node-compatible HMAC-SHA256 token format. Existing valid cookies continue working with the same access key. Strictly parse canonical expiration values and signatures, reject malformed/expired cookies, and use maintained cryptographic primitives with constant-time verification.
- Preserve HttpOnly, SameSite=Strict, path `/`, and Secure on HTTPS. HTTP access is permitted only at loopback development origins. Preserve the redirect that removes the key from the visible URL, `no-store`, and `no-referrer`.
- Access and API capabilities retain their current purpose: the UI/demo can load without a session, while paid research requires authorization. This is not a new account system.
- Require exact configured Origin and expected Host for research before provider work. Reject missing origins and host mismatches. Do not trust arbitrary Forwarded or X-Forwarded headers. The supported Tailscale/proxy path must preserve the external Host; verify that behavior locally and document any explicit trusted-proxy adaptation if required before shipping.
- Bind production and development backend listeners to loopback. Preserve the external app origin and existing Tailscale listener; do not enable Funnel or overwrite port 60090.
- Check content type, body size, JSON syntax, and semantic input before admitting research. Bound incoming body reads to 10 seconds, including bodies without Content-Length. Bound request headers/connections using the supported HTTP server controls; no unbounded waiting room.
- Outbound research calls go only to fixed OpenAI and Tavily HTTPS endpoints with redirects disabled, certificate validation enabled, explicit timeouts, and bounded response reads. Retrieved URLs are citations, never instructions to fetch arbitrary pages on this server. Preserve source-link validation and safe frontend rendering.
- Keep retrieved text and user context as untrusted data in prompts; generation and verification receive no tools. No credentials are included in model inputs. Preserve `store: false` and all existing content-quality constraints.

## Research behavior and provider adapters

Preserve the sequence: validate request; search with bounded ancestor context; normalize/deduplicate sources; generate a structured explanation or clarification; validate the complete topic; check exact evidence quotes/source identities; run the independent grounding verification; return only validated content.

Carry over the existing prompt text and configured model selection, including existing output token budgets. Do not silently choose a different model. Port the existing OpenAI Responses and Tavily request behavior through narrowly scoped HTTP adapters; implementing the Responses wire format, strict JSON schemas, completed/refusal/incomplete handling, and output extraction requires official API documentation during planning/implementation. Avoid coupling the service to a broad unofficial provider SDK.

Retain at most five sources, at most 12,000 UTF-16 units of text per source, server-generated source/paragraph IDs and UTC timestamps, and the current branch/content bounds. Provider raw-response limits are 2 MiB per Tavily/OpenAI response, measured while reading, followed by typed parsing and semantic bounds. Reject oversized or malformed output safely. Invalid source items may be skipped as today; an invalid top-level provider response is a provider failure.

No search results returns `insufficient-evidence` without generation. Clarification and insufficient-evidence results skip topic grounding. Unknown sources, changed URLs, missing evidence, or absent quoted passages prevent a topic from being marked researched. Any unsupported verdict entry rejects the topic; malformed verdicts fail closed. Matching quotes and model review remain checks, not guarantees of factual accuracy.

## Resource limits and lifecycle

Retain these research budgets for one process:

| Resource | Limit |
| --- | --- |
| Request body | 65,536 UTF-8 bytes |
| Incoming body read | 10 seconds |
| Distinct active research jobs | 2 |
| New jobs | 10 per rolling 60 seconds |
| Search stage | 20 seconds within the job deadline |
| Whole research job | 120 seconds from admission |
| Upstream connection establishment | 5 seconds within the remaining stage/job deadline |
| Each upstream response body | 2 MiB |
| HTTP connections | 128 concurrent connections |
| Research waiters | 32 total requests, including original requests |

The additional body/connection/waiter bounds are assistant proposals to close resource-exhaustion gaps, not changes to the paid-research budget. Reject excess admission immediately with 429 and the normal envelope where HTTP processing has begun. Do not queue unbounded jobs or requests. Static and health requests do not consume the two research permits.

Deduplicate the entire validated request, including exploration/node identities and ordered context. Canonical serialization makes equivalent object key ordering irrelevant. One atomic admission decision checks for existing work, available job capacity, the sliding-window budget, and shutdown state. Duplicate waiters share an existing active job without counting as a new paid job, but still count against the waiter bound. Each HTTP response keeps its own request ID; the shared job has a separate job ID for correlation.

The coordinator owns tracked research tasks independently of any one request handler. One caller disconnecting must not cancel other callers' work. When no waiters remain, cancel the local job; do not cache completed results or promise replay on retry. Remove in-flight entries and release capacity on every success, failure, timeout, cancellation, or task panic. Do not retain failed/completed entries. Rate-limit timestamps expire; active maps and metric series stay bounded.

Carry the remaining deadline across provider calls, reads, parsing, and verification. A timeout/cancellation drops local outbound work and prevents later stages; it cannot guarantee the remote provider stops work or refunds a request. Disable automatic HTTP/provider retries. Explicit user retries may incur fresh cost. In-flight deduplication is not durable idempotency or an exactly-once guarantee across restarts.

On SIGINT/SIGTERM, mark readiness false, stop accepting connections/new jobs, and drain existing jobs for up to 125 seconds. At the grace deadline cancel remaining tasks, complete cleanup, flush bounded telemetry, and exit within 130 seconds. Supervisors must allow at least that shutdown window. A second termination signal forces exit. Unexpected task panic is logged safely, converted into a controlled failure for waiters, and cannot strand capacity; fatal startup/supervisor errors exit nonzero.

## Configuration and release operation

Read and validate configuration once at startup. Retain `OPENAI_API_KEY`, `TAVILY_API_KEY`, `OPENAI_MODEL`, `APP_ACCESS_KEY`, `APP_ORIGIN`, `HOST`, and `PORT`; add an explicit frontend asset-directory setting and log-level setting. Preserve the configured/current model default pending a separately authorized model change. Real values remain outside the repository.

Production requires an explicit valid `APP_ORIGIN`, valid loopback binding/port, and a readable frontend build. Reject malformed supplied configuration and supplied access keys shorter than 32 characters without echoing values. Absent access/provider keys remain a supported demo/read-only configuration: app startup succeeds, research returns the existing access/setup errors, and no synthetic live content is substituted. Environment variables override an optional local `.env`; loading it must never print contents or bake secrets into frontend assets.

Document a pinned stable Rust toolchain and existing pinned Node/pnpm build tools; on this Nix system missing commands use comma as instructed. A full flake, containers, Kubernetes, or a new deployment platform is not required. Commit locked dependency builds, minimal features, ignored build output, and CI that builds/tests the release artifact. Verify on Linux CI and the supported macOS host, including termination handling. Audit both committed dependency graphs using Cargo and pnpm ecosystem tools and resolve reachable material advisories before release.

Release instructions cover build, packaged asset/executable placement, configuration, loopback startup, private Tailscale origin, health checks, log lookup, shutdown, and rollback to the prior executable/assets. Preserve the browser-facing origin during upgrade and rollback. Deploying to a new hostname or port is a separate data-migration concern, since IndexedDB is origin-scoped. Do not change active forwarding or stop the existing service merely to prepare this migration.

## Observability and operational endpoints

Operational questions: Is the server serving the app? Is research configured and completing? Which stage or limit caused a failure? Are concurrency and latency within the documented bounds?

- Structured JSON logs carry a timestamp, level, stable event, entry point, request ID, and job ID where applicable. Record stage durations, source counts, safe error codes, admitted/shared/rejected/cancelled jobs, and shutdown transitions.
- Never log full URLs or query strings, access keys, cookies, authorization headers, prompts/questions/context, raw sources, provider bodies, or arbitrary error chains. Do not use default HTTP tracing that records `/access?key=...`. Logging tests exercise authentication failures, timeouts, hostile provider bodies, and unexpected errors.
- Use nested spans for search, generation, deterministic checks, and verification. Correlate shared jobs with each waiting request; do not attribute a shared job to whichever request finishes last. No external telemetry destination is enabled by default.
- Provide bounded-label request counts/errors/duration histograms, stage counts/errors/duration histograms, active jobs/waiters, deduplication counts, and admission rejections. Labels are fixed route templates, stages/providers, status classes, and known error codes, never user IDs, request IDs, raw paths, or error strings.
- `GET /health/live`: minimal public-on-private-listener liveness, no provider calls or secrets.
- `GET /health/ready`: 200 when configured HTTP/static service is ready, 503 when draining/unready. A deliberate demo-only installation is ready; research capability is reported as a boolean and does not test provider credentials or make paid calls.
- `GET /metrics`: existing session authentication, aggregate operational information only, `no-store`. No new metrics password or access flow.
- Update the research runbook with Rust commands, safe diagnostics, readiness interpretation, and rollback. Hosted dashboards, alert routing, and guaranteed SLOs remain outside this single-user migration.

## Verification and acceptance

All routine tests are deterministic and use fixture-backed provider HTTP servers; no real credentials or live research calls are required. Preserve meaningful frontend/domain tests, port Node backend tests to Rust, and delete obsolete implementation-specific tests only after their behavior is covered.

Required evidence before completion:

1. Rust formatting, strict Clippy across targets/features, locked tests/builds, frontend type checking, frontend/domain tests, production build, and both dependency audits pass.
2. Shared JSON fixtures pass Rust validation/serialization and TypeScript schemas, including Unicode limits, success variants, errors, malformed payloads, and boundary sizes. Node-generated fixed cookie fixtures verify in Rust, and Rust cookies match the existing protocol.
3. Real HTTP tests cover authorized/unauthorized access, expiration/tampering, exact origins/hosts, missing/invalid content types, malformed and oversized streamed bodies, slow bodies, unknown routes/methods, safe errors, static-path traversal, cache headers, and security headers.
4. Provider adapter tests inspect exact outgoing wire shapes and exercise valid output, refusals, incomplete/missing output, malicious text, malformed/oversized bodies, 429/5xx, timeouts, and invalid grounding. Assert no hidden retry or unexpected downstream stage.
5. Deterministic coordinator tests cover duplicate races, limits, waiter disconnects, last-waiter cancellation, timeout/panic cleanup, rolling-window expiry, and resumed admission. Shutdown tests cover draining, forced cancellation, and bounded process exit.
6. Existing browser acceptance runs against the built frontend served by Rust, covering branching, source links, refresh failure, interrupted/out-of-order work, reload/resume, phone navigation, persistence failures/conflicts, saved viewport restoration, demos, and independent demo question drafts. Browser mocks can remain for UI scenarios; add at least one production-server path using stub upstream HTTP services so API tests do not all bypass Rust.
7. Demonstrate same-origin upgrade with a preexisting IndexedDB exploration and valid cookie. Saved/demo content remains readable without provider keys. Every demo action makes zero research calls.
8. Capture induced provider failure and timeout in actual structured logs by request ID, check bounded metric labels/counters/histograms, and prove no injected secret/question/raw body appears in telemetry. Verify production hydration/security headers in the browser.
9. Document exactly what was verified and any unavailable platform/dependency checks. Live provider research remains explicitly unverified unless separately authorized; the existing optional two-request smoke may be retained as a manual tool.

## Remaining builder decisions

Planning chooses exact compatible crate versions/features, toolchain pin, module/file names, test HTTP server and metrics implementation, package script composition, CI job layout, and the precise safe static/CSP packaging mechanism. These choices must meet this design's limits and acceptance behavior. No further product decisions are delegated implicitly.

If preserving the current external Host through the actual Tailscale path or meeting static CSP requirements proves impossible without changing observable behavior, surface the concrete constraint before substituting a new auth, origin, or rendering model.

## References checked during design

- [SvelteKit static builds](https://svelte.dev/docs/kit/adapter-static): build-time prerendering and adapter-static configuration.
- [Axum](https://docs.rs/axum/latest/axum/): typed handlers, application state, and Tower integration.
- [Tower HTTP ServeDir](https://docs.rs/tower-http/latest/tower_http/services/struct.ServeDir.html): static serving facilities and path behavior.
- [Reqwest](https://docs.rs/reqwest/latest/reqwest/): asynchronous pooled HTTP client, redirects, and TLS.
- [Tokio shutdown](https://tokio.rs/tokio/topics/shutdown): signal handling, cancellation, and waiting for tasks.
- [Rust 2024](https://doc.rust-lang.org/edition-guide/rust-2024/index.html): stable language edition.

These references support the proposed building blocks; limits, packaging, and scope above are assistant design choices rather than claims that a library supplies all those policies automatically.
