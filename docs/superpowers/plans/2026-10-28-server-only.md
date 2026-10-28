# Server-only Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove browser persistence and demos while retaining server research, saved views and admitted-operation recovery.

**Architecture:** Extend the existing authenticated Rust API with exploration view storage and paginated pending operations. Svelte reads all persistent state from these endpoints; uncertain unadmitted requests live only in memory.

**Tech Stack:** Existing Rust/Axum/SurrealDB, Svelte 5, TypeScript, Vite+, Playwright.

**Spec:** `docs/superpowers/specs/2026-10-28-server-only-design.md`

## Global Constraints

- Preserve the existing uncommitted app/API reorganization; edit the current workspace without committing unrelated work.
- Remove demos and all app-owned browser persistence; leave existing browser databases untouched.
- Use existing authentication, protocol 2 and library scoping. Keep server import APIs/history compatible.
- View writes use independent last-write-wins semantics; research revisions remain unchanged.

## Review Focus

- Stale navigation cannot replace a newer selection during pending recovery or view reads.
- Failed view saves must prevent silent navigation loss and support retry.
- Server outages cannot display a persisted browser copy or enable mutations.
- Pending-operation pagination must not skip records while earlier ones finish.
- Existing imported server answers remain usable without legacy browser migration code.

### Task 1: Server state endpoints

Files: `api/src/{contracts/library,http/library,storage/library,storage/operations,storage/schema}.rs`, `api/tests/library{,_http}.rs`.

Interfaces: GET/PUT `/api/explorations/{id}/view` returns nullable ViewState / saved ViewState. PUT consumes `Command<SaveView>` with explorationId and view. GET `/api/operations?cursor=&limit=` returns Page<OperationView> of pending current-epoch operations, sorted by opaque operation ID.

- [x] Add failing persistence/membership/epoch/pagination/HTTP validation tests; run and observe failure.
- [x] Implement bounded view contracts, transactional writes and paginated operation reads with indexes.
- [x] Run Rust tests and format checks.

### Task 2: Server-only frontend

Files: `app/src/lib/knowledge/{library-client,server-session.svelte,types,schemas}.ts`, `app/src/routes/+page.svelte`, affected components and unit tests.

Interfaces: LibraryClient.view(id), saveView(id, command), operations(cursor). ResearchSession consumes client and onChange only, retaining uncertain intents in memory; uses server pending operations after reload. No cache option.

- [x] Replace storage-specific unit tests with failing server-view, recovery and connection-failure behavior tests.
- [x] Remove cache and demo branches; wire server views/recovery and always load saved explorations from API.
- [x] Remove demo, persistence, migration modules and their obsolete tests. Remove unused browser migration client methods.
- [x] Run frontend tests/check/build.

### Task 3: Browser coverage, dependency and documentation cleanup

Files: affected browser tests, `app/package.json`, `pnpm-lock.yaml`, README and runbooks.

- [x] Adapt browser scenarios to server-backed persistence, absent demos, and cross-browser view restore. Remove tests for retired features; retain research/mobile/conflict coverage.
- [x] Remove idb/fake-indexeddb and refresh lockfile; update active docs and copy.
- [x] Run unit/build/Rust/browser checks and review changes for stale storage paths.

## Execution ledger

Ruling: Work in the existing dirty workspace because the app/API reorganization is necessary context and a clean worktree would omit it. No commits or branch operations are part of this request.

Ruling: Existing session authorization covers implementation after the user's demo removal decision; continue without another approval round.

Task 1: complete — 152 backend tests pass (5 internal/manual fixtures ignored). New view storage persists across reopen, validates membership/epoch, and HTTP boundaries reject invalid inputs.

Task 2: complete — 48 frontend tests pass, including reviewer-reported failed-view navigation/disconnection regressions; frontend check, lint, formatting and production build pass.

Ruling: Retain the server import API and move the digest helper to scripts/legacy-identity.ts because release verification and existing imported server content still require the protocol. The app contains no browser import code.

Review: A fresh reviewer identified rejected view saves surviving discarded explorations and view-read/save network errors leaving mutations enabled. Added failing tests, fixed both, and confirmed the frontend suite passes. Browser checks also exposed duplicate homepage/drawer controls; render the homepage list only while the drawer is closed.

Ruling: Debounce server view writes by 250 ms and coalesce intermediate viewport positions; flush the latest state immediately before navigation. Direct per-frame writes caused avoidable contention with research. The added regression test fails under per-frame writes and passes with coalescing.

Verification: `pnpm check` passes Svelte diagnostics, formatting, lint and TypeScript checks; `cargo clippy --all-targets --all-features --locked -- -D warnings` passes; release build and all 7 packaging/verification tests pass. Browser fixture imports now send serialized fragments through the server API; pagination and purge checks await their actual requests.

Task 3: complete — final full browser suite: 22 passed; frontend: 48 passed; backend: 152 passed (5 internal/manual fixtures ignored); packaging: 7 passed. Production build, Svelte/TypeScript checks, formatting, lint, and Clippy all pass. No IndexedDB calls or idb/fake-indexeddb dependencies remain in app source/manifests/lockfile. Existing user work and browser databases are preserved. Changes remain uncommitted in the current workspace as planned.
