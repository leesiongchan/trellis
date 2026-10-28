# Rust service operation

Trellis runs as one Rust process serving the static Svelte frontend, private access, durable knowledge, research, health and metrics using one embedded SurrealDB/RocksDB handle. Node/pnpm are build and development tools. The packaged executable does not require them at runtime.

## Build and start

Use `mise trust` followed by `mise install --locked node pnpm rust` to install the committed Rust 1.98.0 toolchain (rustfmt/Clippy), Node 26.5.0 and pnpm 11.21.0. Activate mise in the shell or prefix commands with `mise exec --`. Vite+ is installed from the workspace lockfile. The native executable needs none of these development tools at runtime.

```sh
pnpm install --frozen-lockfile --ignore-scripts
node api/scripts/fetch-embedding-model.ts .artifacts/embedding-model
pnpm build
cp .env.example .env
```

Set `APP_DATA_DIR` to an absolute private directory outside the repository, assets, models and backups. See [storage operations](knowledge-storage.md) for native prerequisites and optional backup/model roots. Set the exact browser-facing `APP_ORIGIN` in `.env` and an access key generated with `openssl rand -hex 32`. Add OpenAI/Tavily keys only for live research. Empty keys are a supported read-only configuration. An explicitly supplied access key shorter than 32 UTF-16 units is rejected. Preserve the existing model setting; the default remains `gpt-5.6-terra`, with availability and live operation unverified.

```sh
pnpm start
# Equivalent runtime invocation, with no Node/pnpm dependency:
./release/branch-backend
```

The artifact consists of `release/branch-backend`, `release/public/`, `release/frontend-manifest.json`, `release/release-manifest.json`, `release/native-runtime-manifest.json` and verified `release/models/`. Keep them together. By default assets resolve beside the executable, regardless of the working directory. The optional `.env` resolves from the working directory; `pnpm start` runs from the repository root. For an extracted release, inject environment values or set `APP_ENV_FILE` to an absolute path outside the public directory.

Configuration is read once. Process environment overrides file values. Unset `APP_ENV_FILE` loads `.env` if present; an empty process value disables loading; an explicit path must exist. `FRONTEND_DIR` overrides the asset directory, and the manifest must be its sibling. `HOST` defaults to `127.0.0.1`, `PORT` to 3000; only loopback bindings are accepted. `APP_ORIGIN` is required and must be a root HTTPS origin, or HTTP on loopback. `RUST_LOG` defaults to `info`. Restart after configuration changes. Never print configuration maps or put secrets into command-line arguments, shared logs, build artifacts or browser storage.

`pnpm dev --port 5173` starts Vite+ and the Rust backend after an initial `pnpm build`. Vite+ preserves Host/Origin while forwarding access/API/health/metrics to Rust on port 3001. Dev explicitly uses the loopback frontend origin, independently of the production origin in `.env`. Frontend edits hot reload; restart for Rust changes. A port collision stops the new processes; it does not stop another app.

## HTTPS proxy access and upgrades

The server binds to loopback and can be reached through an HTTPS proxy. The proxy must retain the external Host header; arbitrary `Forwarded` and `X-Forwarded-*` values are not trusted. Set `APP_ORIGIN` to the exact browser-facing origin. Research checks the configured Origin and Host. A local HTTPS proxy fixture verifies this contract.

For deployment, point the proxy at the loopback service. Preserve the existing scheme, hostname and port in `APP_ORIGIN` during upgrades. Keep the access key unchanged if old valid cookies should survive. Changing the key invalidates them.

Explorations and view state live in the server datastore and are available across browsers with private access. The app has no browser cache or offline fallback. Existing browser databases are left untouched and are no longer imported. Back up the server data directory using the supported backup procedure.

Build into a separate release directory for deployment and retain the previous complete executable/assets/manifest. Stop the old service gracefully, switch the release as one unit, and start with the same origin/configuration. Roll back the complete previous artifact with the same procedure. Deploy the protocol-2 frontend and backend together. Startup makes a separate pre-upgrade native backup before a compatible release change; incompatible engine/schema markers stop startup. Retain the old release and its backup. Rollback restores that backup into a fresh, nonexistent data directory and starts its matching old release. Never point an older engine at upgraded live data. See [the restore procedure](knowledge-storage.md#standalone-backup-and-restore).

## Health, limits and shutdown

`GET /health/live` is a minimal process check. `GET /health/ready` returns 200 when the HTTP/static service is ready, including intentional read-only mode, with `researchConfigured` as a boolean. It does not validate provider accounts or make paid calls. During shutdown readiness becomes 503 and new research is rejected. `/metrics` uses the existing private access cookie and returns aggregate OpenMetrics text with no-store.

Research limits per process: two distinct active jobs, ten new jobs per rolling minute, 32 total research waiters, 65,536 request bytes, ten seconds for the complete body, 20 seconds for search, and 120 seconds for the whole job. Each provider connection has a five-second establishment cap and each response a 2 MiB read cap. The server admits 128 connections, 64 headers/32 KiB per header section, and ten seconds for headers; stalled writes time out after 15 seconds. Rejected transport parsing/connection admission may close the socket before an application JSON response exists. API-level errors use the documented envelope.

Identical resolved questions share active work. Durable operation IDs and request hashes replay the original receipt; a changed payload under the same ID conflicts. The server owns admitted work when callers disconnect. Saved-answer reuse is free. Startup marks incomplete paid operations interrupted and resumes only local indexing. Explicit retry may incur a new charge; there are no automatic provider retries. Local cancellation cannot guarantee that a remote provider stops processing or refunds a call.

SIGINT/SIGTERM stops acceptance and drains existing connections/jobs for up to 125 seconds. Remaining work is cancelled and the process must exit by 130 seconds. Supervisors must allow at least 130 seconds; a second termination signal forces exit. Check the request/job logs before escalating a stalled shutdown. Do not kill unrelated processes by port or process-name matching.

Static startup verifies manifest version, safe paths, regular files, SHA-256 digests and script-hash syntax. Asset reads use descriptor-relative no-follow operations and then an in-memory snapshot. Symlink escapes, missing/tampered assets and missing index/manifest fail startup. Packaging excludes dotfiles, source maps, source code and test certificates. Bounds are 4 MiB for the manifest, 4,096 files, 32 MiB per file and 128 MiB in aggregate. Immutable hashed assets cache for a year; HTML/build metadata revalidate. CSP includes exact inline-script hashes; it does not allow inline scripts or eval generally.

## Diagnostics and verification

Use the reference ID in the UI to find structured `requestId` events. `research_waiting` connects a request to its `jobId`; stage spans/events report search, generation, grounding and verification duration/outcome. Shared callers retain separate request IDs. Request/response bodies, questions, raw sources, cookies, keys and access-link query strings must not appear in telemetry.

Metrics include HTTP/stage counts, failures, duration histograms, active jobs/waiters, shared requests and admission rejections. Labels use fixed route/stage/status/code sets. Request IDs and raw paths are not labels. There is no hosted collector, dashboard or alert routing requirement for this private service.

JSON output uses a dedicated writer thread with a 256-event queue and a 16 KiB event cap. A full queue, oversized event or output error drops the event and increments `branch_log_events_dropped_total`; HTTP workers and emergency signal handling never wait on the collector. Normal termination attempts to flush for at most 100 ms. A stalled collector can lose final events, including shutdown completion; use process status/readiness as well as logs.

`RUST_LOG` controls application verbosity, defaulting to `info`. Only the exact `branch_backend` target and its `branch_backend::` module targets are admitted. An independent filter always suppresses dependency events and spans, including their context fields, even with global `debug`/`trace` or explicit dependency directives. Use application module directives such as `branch_backend::providers=debug` for focused diagnostics. Isolated subprocess tests invoke the production initializer and check redaction, correlation, verbosity and dependency-span exclusion.

Install the optional audit CLI once with `mise exec -- cargo install cargo-audit --version 0.22.2 --locked` before running `cargo audit` below. CI installs the same pinned version.

```sh
cargo fmt --manifest-path api/Cargo.toml -- --check
cargo clippy --manifest-path api/Cargo.toml --all-targets --all-features --locked -- -D warnings
cargo test --manifest-path api/Cargo.toml --all-targets --all-features --locked
pnpm check
pnpm test:unit
pnpm build
pnpm test:build
pnpm test:e2e
cargo audit --file api/Cargo.lock
pnpm audit
```

Tests and release verification use only fake credentials/local providers. Browser tests own ports 5183–5187 and never reuse a running app. The fixture harnesses are explicitly ignored tests, not a production endpoint override. Release verification starts its own server on a temporary port with an empty child PATH and no provider keys. The optional `pnpm live-smoke` remains manual, makes at most two paid requests, and is excluded from routine verification.

The release verifier uses only temporary private storage and the packaged native executable, with no Node/Python/container/database CLI on its child PATH. It covers v1 import, SIGKILL/reopen, native CLI backup/restore and valid/missing/corrupt model assets. The separate native crash fixture proves pending paid research and verifier stages become interrupted without provider replay.

See [research diagnostics](research.md) for error meanings and [migration evidence](../superpowers/2026-10-26-rust-backend-notes.md) for actual verification scope.
