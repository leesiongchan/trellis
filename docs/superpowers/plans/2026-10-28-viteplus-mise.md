# Vite+ and mise migration

Approved direction: adopt Vite+ incrementally for the existing Svelte frontend, preserving Cargo, Svelte diagnostics, Playwright, and release acceptance. Use mise instead of Nix for project tool versions. The existing app/ and api/ relocation is part of the working tree and must survive migration.

- [x] Pin Node 26.5.0, pnpm 11.21.0 and Rust 1.98.0 with rustfmt/Clippy in mise.toml; leave machine/global runtime configuration unchanged.
- [x] Run the release-pinned Vite+ migrator from the workspace root, review every warning and preserve the existing Svelte compiler mode, aliases and unit-test discovery.
- [x] Consolidate frontend build/test configuration in app/vite.config.ts; keep Svelte diagnostics and Playwright as explicit tasks. Adapt the combined dev supervisor to launch the workspace Vite+ executable without changing origin, process teardown, or private data handling.
- [x] Use mise in CI and setup documentation; preserve backend tests, checksummed assets and packaging acceptance. Avoid task caching for live-provider checks and release acceptance.
- [x] Verify locked install, frontend checks, 76 existing unit tests, static build, package/runtime checks, browser discovery and development startup. Report any native rebuild/browser execution not performed.

Use existing behavioral checks for this tooling change; add a regression only if execution exposes an uncovered behavior. No provider calls, deployment, global tool replacement or commits are authorized by this migration.

## Verification

- Vite+ 1.4.0 migrator completed. Its only warning was the legacy .prettierignore; it is now removed and its exclusions live in root fmt.ignorePatterns.
- Existing Vitest was already 5.0.5; no v4 compatibility settings were generated. No tsdown/library migration applies.
- Locked mise install and frozen pnpm install passed. Node 26.5.0, pnpm 11.21.0 and Cargo 1.98.0 were selected through mise. mise.lock records the Linux x64 and macOS arm64 downloads.
- Svelte diagnostics, formatting and type-aware lint passed without warnings. A narrow fixture-only no-base-to-string override covers fake fetch URL arguments; Node test registrations explicitly discard framework-owned promises.
- All 76 unit tests in 13 files passed; Vite+ production frontend build passed.
- All 7 package tests and the native release verifier passed with the new frontend and the previously verified unchanged native executable.
- Two existing desktop/phone demo tests passed through the actual Vite+ dev server and native API proxy using disposable storage. An owned test-only Cargo shim validated api/Cargo.toml arguments and launched the existing executable instead of recompiling it. Direct supervisor termination closed both ports. No provider calls occurred.
- Both full browser configurations still discover their existing tests; production discovery reports 27. The full 27-test fixture suite and native recompilation were not repeated for this tooling migration.
- Hosted macOS/Linux CI remains unexecuted locally.

CI workflow syntax passed actionlint 1.7.12 and shellcheck 0.11.0 selected through mise. The dependency audit remains nonzero for one low-severity advisory, GHSA-gcqx-w48m-w2h4, in deepmerge 4.3.1 through the Svelte Vite plugin. A targeted recursive update retained 4.3.1; although the audit response lists >=4.3.2 as patched, the npm registry has no deepmerge 4.3.2 release. No audit exception or severity-gate weakening was added. This build-tool dependency is absent from the packaged Rust/static runtime.

Focused independent migration review found no Critical/Important issues. Its sole Minor finding (missing cargo-audit setup instructions) was fixed in README and the Rust service runbook.
