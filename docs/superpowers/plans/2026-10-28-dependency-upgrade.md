# Latest dependency ranges implementation plan

> **For agentic workers:** Use superpowers:executing-plans to carry out these maintenance steps in the current workspace.

**Goal:** Update all first-party JavaScript and Rust dependencies to the latest published releases with explicit caret ranges, preserving a working build.

**Architecture:** Keep the pnpm workspace, shared Vite+ catalog and embedded Rust service. Retain vendored upstream manifests and pinned development runtimes. Install TypeScript 7 alongside the official TypeScript 6 compatibility API required by the latest Svelte tooling. Keep the existing ort release candidate because no stable ort version is published.

**Tech Stack:** pnpm, Vite+, Svelte, TypeScript, Cargo, Axum and SurrealDB.

**Spec:** User request: “bump all deps to latest with caret”. Registry metadata was fetched from registry.npmjs.org and crates.io on 2026-10-28; direct dependencies and both lockfiles are in scope.

## Global constraints

- Preserve the uncommitted Tailscale documentation cleanup.
- Use caret ranges for versioned first-party dependency declarations and catalog entries; keep catalog references and the Vite+ alias.
- Keep dependency install scripts disabled and do not change authentication behavior, stored data, runtime tool pins, vendored code or native model assets.
- Use current stable releases; retain the existing ort prerelease family because it has no stable release.

## Review focus

- Existing signed cookies must remain valid after HMAC/SHA-2/base64 upgrades.
- Both current and newly issued library data must preserve identity, backup compatibility and evidence encoding.
- Svelte diagnostics must still work with the TypeScript 6 compiler API while TypeScript 7 is installed.
- Shared catalog ranges and peer dependencies must resolve without switching away from Vite+.
- Regenerated lockfiles and platform-native optional packages must work without enabling arbitrary install scripts.

## Task 1: Update manifests and lockfiles

Files: `package.json`, `app/package.json`, `pnpm-workspace.yaml`, `api/Cargo.toml`, `pnpm-lock.yaml`, `api/Cargo.lock`.

- [x] Query official registry latest tags/releases and inspect peer/runtime requirements.
- [x] Apply caret ranges and the documented TypeScript compatibility aliases.
- [x] Refresh pnpm/Cargo lockfiles and inspect package, script and peer changes.

## Task 2: Resolve compatibility and verify

Files: existing production modules only if the upgraded APIs require changes; README tooling notes.

- [x] Run existing checks first; use actual failures to identify any required API adaptations.
- [x] Keep cookie bytes, digest outputs, storage format and provider behavior unchanged.
- [x] Run frontend checks/unit tests, Rust checks/tests, build, release checks and browser acceptance.
- [x] Run native dependency audits when available and record results or concrete limitations.
- [x] Compare first-party manifests against the registry inventory and verify caret coverage.

## Execution notes

Registry findings: Svelte 5.57.3, TypeScript 7.0.2, base64 0.23.1, hmac 0.13.0 and sha2 0.11.1 are updates. The official `@typescript/typescript6` compatibility package is 6.0.2; SvelteKit 3.1.0 and svelte-check 4.7.7 still require the 6.x API. All other direct dependencies are already at their current registry release; update their lower bounds/caret syntax explicitly. ort remains 2.0.0-rc.15, the latest published version with no stable release.

Compatibility: the initial upgraded Rust check failed because HMAC 0.13 exposes initialization through KeyInit and SHA-2 0.11 outputs no longer implement LowerHex. Import KeyInit and use the current hex 0.4.3 encoder for byte-identical lowercase SHA-256 strings, including fixtures. All-targets cargo check then passed, as did existing cookie/identity/backup/storage tests.

Registry and lockfile verification: 36 first-party Rust dependency declarations use caret ranges at the latest registry release, including the added hex encoder and duplicated development dependencies. JavaScript ranges match latest registry tags through direct ranges, aliases and catalog entries. Native TypeScript reports 7.0.2. Vendored upstream and runtime pins remain unchanged.

Audit: pnpm reports one low-severity advisory, GHSA-gcqx-w48m-w2h4, for deepmerge 4.3.1 used by the Svelte Vite plugin to merge repository-owned build configuration. Registry latest is still 4.3.1 and the upstream advisory lists no published fix; do not invent an unavailable override or change application behavior to silence the audit. The unchanged CI audit step therefore remains nonzero. Cargo audit reports zero non-ignored vulnerabilities under the existing reviewed exclusions and informational unmaintained atomic-polyfill/paste warnings. No audit exclusions were added.

Review: a fresh reviewer found no actionable dependency/compatibility issues; checked aliases, executable shims, caret coverage and unchanged HMAC inputs/digest representation.

Final-lockfile validation: frontend diagnostics, format, lint and types passed; 48 frontend unit tests, 152 native tests, seven packaging tests and 22 browser tests passed (229 total; five native fixture/manual tests ignored). Cargo format and Clippy with warnings denied passed. The optimized production build and release packaging passed. Browser acceptance covered legacy Node-signed cookies, persisted server data across upgrades, graph navigation and fixture-backed research over HTTP and HTTPS.
