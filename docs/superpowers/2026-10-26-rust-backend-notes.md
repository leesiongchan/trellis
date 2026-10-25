# Rust backend migration evidence

The approved design and implementation plan are implemented on `feat/rust-backend`. The runtime is one Rust executable plus static frontend assets. The Svelte UI, browser storage schema/database, configured model default, private access protocol and provider-free demo remain compatible.

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

1. Use the writable checkout on a feature branch after the managed worktree was created outside permitted write roots. `main` remains unchanged; protection from simultaneous file edits is weaker than a separate worktree. The unused native worktree has no implementation changes.
2. Preserve the installed Zod version's code-point string limits, while the custom ancestor-total check and legacy source clipping use UTF-16. The plan's blanket UTF-16 assumption failed the cross-language fixture. Following it would have rejected previously valid multilingual questions.
3. Make the Tokio socket test helper asynchronous. This corrects the plan's omitted async requirement and changes no product API.
4. Run independent HTTP/HTTPS fixture instances concurrently on backend ports 5184/5186, with the HTTPS proxy on 5185. Each app still has one exact configured origin. This adds one test-only loopback listener and avoids restarts during the suite.

## Independent review and fix

One fresh-context whole-branch review used `gpt-6-astra`, covering `dc1f24a..60f5f8f`, the approved spec/plan and the four implementation rulings. The reviewer found no Critical issues, one Important availability defect and one Minor logging-boundary concern. It judged the five review-focus areas and the implementation rulings sound. Hosted CI, live providers, actual Tailscale deployment and physical-device behavior were explicitly excluded from claims.

The Important defect was synchronous stdout logging on Tokio workers: an unread pipe could stall health and signal handling. `unread_log_sink_preserves_health_and_bounded_termination` reproduced the health timeout before the fix, then passed. Output now uses a dedicated thread, 256-event nonblocking queue, 16 KiB event cap, overflow/output-error counter and at most 100 ms shutdown flush. The real-process regression saturates output, verifies continued health and observable drops, then checks ordinary SIGTERM and second-signal exit with a pending upload. It never drains stdout. No dependency or production endpoint changed. The fix receives full-suite verification rather than another reviewer.

Deferred minor: unrestricted `RUST_LOG` can enable dependency diagnostics beyond the intended application event allowlist. Existing tests do not exercise the production subscriber at debug/trace. No credential, question, cookie or provider-body leak was established; Hyper internal tracing is compiled out and Reqwest byte logging is disabled. The runbook recommends normal `info` operation and records the limitation.

## Final review rulings

5. Keep live OpenAI/Tavily behavior, model availability and research quality unverified. Paid calls were excluded; fixtures establish protocol behavior. Cost if wrong: a configured project/model can fail or produce poor research despite passing local fixtures.
6. Keep the actual Rust-through-Tailscale path unverified. Deployment/restart was not authorized, and forwarding stays unchanged. Cost if wrong: the actual proxy may require investigation before deployment.
7. Use browser phone emulation as the available mobile evidence; a physical phone remains unverified. Cost if wrong: device-specific layout/storage behavior may differ.
8. Do not claim hosted Linux CI success from the workflow. It passed actionlint locally, but the hosted job has not run. Cost if wrong: a platform/runner issue may still surface.
9. Retain the single-user private-process operating model. Public hosting, multiple users, distributed coordination and server persistence were explicitly excluded. Cost if wrong: this release is unsuitable for an unplanned public/shared deployment.
