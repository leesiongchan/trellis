# Rust service operation

Branch runs as one Rust process serving the static Svelte frontend, private access, research, health and metrics. Node/pnpm are build and development tools. The packaged executable does not require them at runtime.

## Build and start

Use the committed Rust 1.98.0 toolchain (rustfmt/Clippy), Node 26.5.0 and pnpm 11.21.0. Node 24+ remains the declared build floor; verification uses 26.5.0. On Nix, retry missing tools with comma, such as `, cargo-audit audit --file backend/Cargo.lock`.

```sh
pnpm install --frozen-lockfile --ignore-scripts
pnpm build
cp .env.example .env
```

Set the exact browser-facing `APP_ORIGIN` in `.env` and an access key generated with `openssl rand -hex 32`. Add OpenAI/Tavily keys only for live research. Empty keys are a supported demo/read-only configuration. An explicitly supplied access key shorter than 32 UTF-16 units is rejected. Preserve the existing model setting; the default remains `gpt-5.6-terra`, with availability and live operation unverified.

```sh
pnpm start
# Equivalent runtime invocation, with no Node/pnpm dependency:
./release/branch-backend
```

The artifact consists of `release/branch-backend`, `release/public/`, and `release/frontend-manifest.json`. Keep them together. By default assets resolve beside the executable, regardless of the working directory. The optional `.env` resolves from the working directory; `scripts/start.sh` first enters the repository root. For an extracted release, inject environment values or set `APP_ENV_FILE` to an absolute path outside the public directory.

Configuration is read once. Process environment overrides file values. Unset `APP_ENV_FILE` loads `.env` if present; an empty process value disables loading; an explicit path must exist. `FRONTEND_DIR` overrides the asset directory, and the manifest must be its sibling. `HOST` defaults to `127.0.0.1`, `PORT` to 3000; only loopback bindings are accepted. `APP_ORIGIN` is required and must be a root HTTPS origin, or HTTP on loopback. `RUST_LOG` defaults to `info`. Restart after configuration changes. Never print configuration maps or put secrets into command-line arguments, shared logs, build artifacts or browser storage.

`pnpm dev --port 5173` starts Vite and the Rust backend after an initial `pnpm build`. Vite preserves Host/Origin while forwarding access/API/health/metrics to Rust on port 3001. Dev explicitly uses the loopback frontend origin, independently of the production origin in `.env`. Frontend edits hot reload; restart for Rust changes. A port collision stops the new processes; it does not stop another app.

## Private Tailscale access and upgrades

Inspect `tailscale serve status` before changing a listener. Existing forwarding and preview port 60090 must be preserved. This migration does not alter forwarding or enable Funnel. The supported proxy must retain the external Host header; arbitrary `Forwarded` and `X-Forwarded-*` values are not trusted. Research checks the exact configured Origin and Host. A local HTTPS proxy fixture verifies this contract; the new Rust release has not been deployed through the actual tailnet route.

For a separately authorized deployment, preserve the existing scheme, hostname and port in `APP_ORIGIN` and point the existing app route at the loopback service. Keep the access key unchanged if old valid cookies should survive. Changing the key invalidates them. Do not create a second route or overwrite preview forwarding to perform a routine binary upgrade.

IndexedDB is scoped to the complete browser origin. Changing a hostname or port makes previous explorations inaccessible at the new origin. No server copy, cross-device synchronization or automatic browser backup exists. Do not clear site data to repair startup, research or saving failures.

Build into a separate release directory for deployment and retain the previous complete executable/assets/manifest. Stop the old service gracefully, switch the release as one unit, and start with the same origin/configuration. Roll back the complete previous artifact with the same procedure. There is no server database migration; in-flight jobs are not durable or replayed across restarts.

## Health, limits and shutdown

`GET /health/live` is a minimal process check. `GET /health/ready` returns 200 when the HTTP/static service is ready, including intentional read-only mode, with `researchConfigured` as a boolean. It does not validate provider accounts or make paid calls. During shutdown readiness becomes 503 and new research is rejected. `/metrics` uses the existing private access cookie and returns aggregate OpenMetrics text with no-store.

Research limits per process: two distinct active jobs, ten new jobs per rolling minute, 32 total research waiters, 65,536 request bytes, ten seconds for the complete body, 20 seconds for search, and 120 seconds for the whole job. Each provider connection has a five-second establishment cap and each response a 2 MiB read cap. The server admits 128 connections, 64 headers/32 KiB per header section, and ten seconds for headers; stalled writes time out after 15 seconds. Rejected transport parsing/connection admission may close the socket before an application JSON response exists. API-level errors use the documented envelope.

Identical validated requests share active work. One disconnected caller does not cancel other callers; the last departing waiter cancels local work. Later retries can incur a new charge. Local cancellation cannot guarantee that a remote provider stops processing or refunds a call. There are no automatic provider retries and no completed-job cache on the server.

SIGINT/SIGTERM stops acceptance and drains existing connections/jobs for up to 125 seconds. Remaining work is cancelled and the process must exit by 130 seconds. Supervisors must allow at least 130 seconds; a second termination signal forces exit. Check the request/job logs before escalating a stalled shutdown. Do not kill unrelated processes by port or process-name matching.

Static startup verifies manifest version, safe paths, regular files, SHA-256 digests and script-hash syntax. Asset reads use descriptor-relative no-follow operations and then an in-memory snapshot. Symlink escapes, missing/tampered assets and missing index/manifest fail startup. Packaging excludes dotfiles, source maps, source code and test certificates. Bounds are 4 MiB for the manifest, 4,096 files, 32 MiB per file and 128 MiB in aggregate. Immutable hashed assets cache for a year; HTML/build metadata revalidate. CSP includes exact inline-script hashes; it does not allow inline scripts or eval generally.

## Diagnostics and verification

Use the reference ID in the UI to find structured `requestId` events. `research_waiting` connects a request to its `jobId`; stage spans/events report search, generation, grounding and verification duration/outcome. Shared callers retain separate request IDs. Request/response bodies, questions, raw sources, cookies, keys and access-link query strings must not appear in telemetry.

Metrics include HTTP/stage counts, failures, duration histograms, active jobs/waiters, shared requests and admission rejections. Labels use fixed route/stage/status/code sets. Request IDs and raw paths are not labels. There is no hosted collector, dashboard or alert routing requirement for this private service.

JSON output uses a dedicated writer thread with a 256-event queue and a 16 KiB event cap. A full queue, oversized event or output error drops the event and increments `branch_log_events_dropped_total`; HTTP workers and emergency signal handling never wait on the collector. Normal termination attempts to flush for at most 100 ms. A stalled collector can lose final events, including shutdown completion; use process status/readiness as well as logs.

`RUST_LOG` controls application verbosity, defaulting to `info`. Only the exact `branch_backend` target and its `branch_backend::` module targets are admitted. An independent filter always suppresses dependency events and spans, including their context fields, even with global `debug`/`trace` or explicit dependency directives. Use application module directives such as `branch_backend::providers=debug` for focused diagnostics. Isolated subprocess tests invoke the production initializer and check redaction, correlation, verbosity and dependency-span exclusion.

```sh
cargo fmt --manifest-path backend/Cargo.toml -- --check
cargo clippy --manifest-path backend/Cargo.toml --all-targets --all-features --locked -- -D warnings
cargo test --manifest-path backend/Cargo.toml --all-targets --all-features --locked
pnpm check
pnpm test:unit
pnpm build
pnpm test:build
pnpm test:e2e
pnpm verify:release
, cargo-audit audit --file backend/Cargo.lock
pnpm audit
```

Tests and release verification use only fake credentials/local providers. Browser tests own ports 5183–5186 and never reuse a running app. The fixture harness is an explicitly ignored test, not a production endpoint override. Release verification starts its own server on a temporary port with an empty child PATH and no provider keys. The optional `pnpm live-smoke` remains manual, makes at most two paid requests, and is excluded from routine verification.

See [research diagnostics](research.md) for error meanings and [migration evidence](../superpowers/2026-10-26-rust-backend-notes.md) for actual verification scope.
