# Rust backend migration evidence

The approved design and implementation plan were implemented on `feat/rust-backend` and merged into `main` at the user's request. The runtime is one Rust executable plus static frontend assets. The Svelte UI, browser storage schema/database, configured model default, private access protocol and provider-free demo remain compatible.

## Verification recorded on 2026-10-26

- macOS arm64: Rust formatting, strict Clippy, all backend targets, frontend type checking and production build passed during implementation.
- 44 backend tests pass; the browser fixture-server test is intentionally ignored in normal Cargo runs and launched only by the browser supervisor.
- 43 frontend/domain/contract tests and four build/release-verifier tests cover the final implementation. Release-verifier red/green: all three new checks failed against the artifact-only stub before runtime verification was added, then passed.
- 21 production browser scenarios passed after Node code/dependency removal. They include the existing 17 scenarios, one production upgrade scenario, fixture-backed research over HTTP and HTTPS, and HTTPS upgrade compatibility.
- A named agent-browser session additionally opened the Rust-served homepage and expanded the provider-free demo to the memory-cell/quantum-tunneling branch. The session was closed after inspection.
- The executable served the packaged frontend/readiness from an unrelated working directory and exited cleanly on SIGTERM. The automated verifier also removes Node from the child's PATH, checks access/metrics, setup-required responses, content hashes/CSP and secret redaction, and rejects missing/Node-dependent executables.
- A sanitized development-supervisor smoke on ports 5194/3001 verified Vite-to-Rust access, preserved Host/Origin and setup-required behavior; its owned children stopped on SIGTERM.
- pnpm audit reported zero known vulnerabilities across 122 dependencies. Cargo audit 0.22.2 scanned 217 locked crate dependencies against 1,352 loaded RustSec advisories and reported no vulnerabilities.
- Tailscale Serve status was inspected read-only: an existing private HTTPS listener on 8443 was present. Forwarding was not changed. Native Rust through that actual route remains unverified because this task does not deploy or restart the existing service.

Linux CI is configured, not claimed to have run. A separate physical phone, live OpenAI/Tavily behavior and configured model availability remain unverified. No paid provider calls were made. The local HTTPS fixture verifies Host preservation and Secure-cookie behavior; it is not evidence of a live tailnet deployment.

Final command results on macOS arm64:

| Command | Observed result |
| --- | --- |
| `pnpm install --frozen-lockfile --ignore-scripts` | Exit 0, lockfile current |
| `cargo fmt --manifest-path backend/Cargo.toml -- --check` | Exit 0 |
| `cargo clippy --manifest-path backend/Cargo.toml --all-targets --all-features --locked -- -D warnings` | Exit 0 |
| `cargo test --manifest-path backend/Cargo.toml --all-targets --all-features --locked` | 44 passed, zero failed; one intentionally ignored fixture harness |
| `pnpm check` | Zero errors/warnings |
| `pnpm test:unit` | 43 passed |
| `pnpm build` | Static frontend, locked optimized Rust build and release packaging passed |
| `pnpm test:build` | Four passed |
| `pnpm test:e2e` | 21 passed after the review fix |
| `node scripts/verify-release.ts` | `ok: true`, all eight runtime/integrity/redaction checks reported |
| `, cargo-audit audit --file backend/Cargo.lock` | Exit 0, no vulnerabilities in 217 locked crates |
| `pnpm audit --json` | Exit 0, zero known vulnerabilities in 122 dependencies |
| `, actionlint .github/workflows/ci.yml` | Exit 0, actionlint 1.7.12 |
| `git diff --check` | Exit 0 |

Tasks 1–9 are recorded in commits `4478c3e` through `4fbce95`; Task 10 is `60f5f8f`, with the reviewed logging fix in `fdbadb3`. The final completion gate reran the full backend suite successfully after that fix. The execution scratch directory was removed after preserving this evidence, review outcome and all rulings here.

On the user's subsequent merge/cleanup instruction, `main` fast-forwarded to `a71a389`. Post-merge checks passed: 44 backend tests, 43 frontend tests, four build/release tests, 21 production browser scenarios and frontend type checking. The fully merged feature branch and clean unused detached worktree were removed without force. Only `main` and the primary checkout remain. No Git remote is configured; this was a local merge, with no push or deployment.

## Replacement test coverage

| Removed Node tests | Rust replacement |
| --- | --- |
| `tests/integration/access.test.ts` | `backend/tests/access.rs` and fixed Node cookie fixtures |
| `tests/integration/research-api.test.ts` | `backend/tests/http.rs`, `jobs.rs`, `lifecycle.rs`, `telemetry.rs` |
| `tests/unit/retrieval.test.ts` | `backend/tests/providers.rs` |
| `tests/unit/openai.test.ts` | `backend/tests/providers.rs` and fixture browser research |
| `tests/unit/research.test.ts` | `backend/tests/research.rs` |
| `tests/unit/grounding.test.ts` | `backend/tests/research.rs` |

All frontend/domain tests remain. The old OpenAI JavaScript SDK and adapter-node dependency are removed. Provider mocks are confined to tests; the release executable constructs only fixed production endpoints. The test certificate/private key are public fixture data and are excluded from release assets.

The final browser run exposed an existing recovery-test race: the refresh-only error mock could intercept the initial NAND request before it completed. The trace showed no prior content to refresh. The test now waits for the Memory cells branch before installing the failure; the focused regression passed. No application behavior changed.

## Rulings made during implementation

1. Use the writable checkout on a feature branch after the managed worktree was created outside permitted write roots. During implementation, `main` remained unchanged; protection from simultaneous file edits was weaker than a separate worktree. The unused native worktree had no implementation changes and was removed during the later authorized cleanup.
2. Preserve the installed Zod version's code-point string limits, while the custom ancestor-total check and legacy source clipping use UTF-16. The plan's blanket UTF-16 assumption failed the cross-language fixture. Following it would have rejected previously valid multilingual questions.
3. Make the Tokio socket test helper asynchronous. This corrects the plan's omitted async requirement and changes no product API.
4. Run independent HTTP/HTTPS fixture instances concurrently on backend ports 5184/5186, with the HTTPS proxy on 5185. Each app still has one exact configured origin. This adds one test-only loopback listener and avoids restarts during the suite.

## Independent review and fix

One fresh-context whole-branch review used `gpt-6-astra`, covering `dc1f24a..60f5f8f`, the approved spec/plan and the four implementation rulings. The reviewer found no Critical issues, one Important availability defect and one Minor logging-boundary concern. It judged the five review-focus areas and the implementation rulings sound. Hosted CI, live providers, actual Tailscale deployment and physical-device behavior were explicitly excluded from claims.

The Important defect was synchronous stdout logging on Tokio workers: an unread pipe could stall health and signal handling. `unread_log_sink_preserves_health_and_bounded_termination` reproduced the health timeout before the fix, then passed. Output now uses a dedicated thread, 256-event nonblocking queue, 16 KiB event cap, overflow/output-error counter and at most 100 ms shutdown flush. The real-process regression saturates output, verifies continued health and observable drops, then checks ordinary SIGTERM and second-signal exit with a pending upload. It never drains stdout. No dependency or production endpoint changed. The fix receives full-suite verification rather than another reviewer.

Initially deferred minor: unrestricted `RUST_LOG` could enable dependency diagnostics beyond the intended application event allowlist. The original tests did not exercise the production subscriber at debug/trace. No credential, question, cookie or provider-body leak was established; Hyper internal tracing is compiled out and Reqwest byte logging is disabled. This follow-up was subsequently authorized and resolved below.

## Logging follow-up completed on 2026-10-26

At the user's instruction to do only the first next step, the production subscriber now applies an independent metadata filter admitting only `branch_backend` and `branch_backend::` targets. Dependency events and spans stay disabled even when `RUST_LOG` explicitly enables them. Application verbosity/module directives still work; queued output, bounds and shutdown behavior are unchanged.

The redaction test now launches isolated subprocesses that call the actual production initializer and queued writer. Four configurations cover global debug, global trace, explicit dependency directives alongside application debug, and dependency-only directives with application logging off. Injected dependency error/body/span fields and a similarly prefixed fake target are rejected. A real local provider-failure request verifies request/job correlation, safe codes and absence of fake credentials, signed cookies, question/context, access-query values and raw provider/source text in stdout, stderr and metrics. The regression first failed on `private-dependency-error` under debug, then passed after filtering; dependency span context is excluded from allowed application events.

Verification: formatting and strict all-target/all-feature Clippy passed; the locked backend suite passed 44 tests. Two fixture helpers are ignored in the outer suite: the production logging helper is explicitly executed by its parent test, and the browser server remains reserved for browser acceptance. The optimized locked backend build, packaging and all eight native release smoke checks passed. No dependencies or frontend behavior changed. Remote setup, hosted Linux CI, deployment, forwarding changes, physical-phone checks and paid provider calls were not performed in this follow-up.

## Final review rulings

5. Keep live OpenAI/Tavily behavior, model availability and research quality unverified. Paid calls were excluded; fixtures establish protocol behavior. Cost if wrong: a configured project/model can fail or produce poor research despite passing local fixtures.
6. Keep the actual Rust-through-Tailscale path unverified. Deployment/restart was not authorized, and forwarding stays unchanged. Cost if wrong: the actual proxy may require investigation before deployment.
7. Use browser phone emulation as the available mobile evidence; a physical phone remains unverified. Cost if wrong: device-specific layout/storage behavior may differ.
8. Do not claim hosted Linux CI success from the workflow. It passed actionlint locally, but the hosted job has not run. Cost if wrong: a platform/runner issue may still surface.
9. Retain the single-user private-process operating model. Public hosting, multiple users, distributed coordination and server persistence were explicitly excluded. Cost if wrong: this release is unsuitable for an unplanned public/shared deployment.
