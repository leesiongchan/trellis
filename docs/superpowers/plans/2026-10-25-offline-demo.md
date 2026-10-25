# Offline Demo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Make the approved branching experience usable with prewritten microSD content and no provider calls.

**Architecture:** Add a version-compatible exploration mode and a factory for a complete local tree. Guard research in the session; expose launch, persistent labels and unanswered custom questions through existing Svelte components.

**Tech Stack:** Existing Svelte 5, SvelteKit 3, Zod, IndexedDB, Vitest and Playwright. No dependencies added.

**Spec:** docs/superpowers/specs/2026-10-25-offline-demo-design.md

## Global Constraints

- No provider requests, credentials, dependency additions, server endpoints, public hosting, cross-device synchronization or export/import.
- Default legacy saved explorations to `research`.
- Every demo launch creates a new copy; never overwrite existing work.
- New custom questions remain unanswered and local.
- Existing research behavior remains available outside demo mode.

## Review Focus

- Legacy saved records lacking mode remain readable: task 1 schema regression.
- Refresh and clarification accidentally invoking research: task 1 session regression.
- Custom branches silently inventing explanations or losing input: task 1 domain and task 2 reload checks.
- Demo state leaking when switching to a research exploration: task 1 real-mode check and task 2 navigation check.
- Phone overflow and unclear sample provenance: task 2 phone test and visual inspection.

### Task 1: Demo domain and research boundary

**Files:** Create `src/lib/knowledge/demo.ts`, `tests/unit/demo.test.ts`; modify `schemas.ts`, `session.svelte.ts`, and `tests/fixtures/knowledge.ts`.

**Interfaces:** Produce `createDemoExploration(): Exploration` with fresh ID, mode `demo`, eight ready linked nodes and root expanded. Exploration mode defaults to `research`. Consume existing `KnowledgeSession.load`, `expand`, `select`, `refresh`, `clarify` and `addQuestion` APIs.

- [x] Write tests: two factory calls have different exploration IDs and independent nodes; schema validates tree and restores missing mode as research; demo expand/refresh/clarify/custom question never invokes ResearchClient; custom question persists as suggested with no content; duplicate questions reuse nodes; research mode still invokes ResearchClient.
- [x] Run `pnpm exec vitest run tests/unit/demo.test.ts`; expect FAIL for the missing factory/mode/guard.
- [x] Implement bundled primary-source content, mode migration default and an early research guard for demo sessions. Preserve existing input validation and normal research calls.
- [x] Run `pnpm test:unit`; expect all tests PASS. Run `pnpm check`; expect zero errors/warnings.
- [x] Commit domain changes.

### Task 2: Demo launch, reader, persistence and handoff

**Files:** Modify `src/routes/+page.svelte`, `TopicStart.svelte`, `Workspace.svelte`, `TopicReader.svelte`, `ExplorationList.svelte`, `src/app.css`, `README.md`; create `tests/e2e/demo.spec.ts`.

**Interfaces:** Consume `createDemoExploration()` and persisted `exploration.mode`. Add `onDemo: () => void` to TopicStart and `demo?: boolean` to TopicReader. Page launch flushes current work before loading and enqueuing the new copy.

- [x] Write browser tests for launch, three levels, source link, collapse/reopen, custom question, reload/resume, persistent labels, zero research requests and phone map/reader navigation.
- [x] Run `pnpm exec playwright test tests/e2e/demo.spec.ts`; expect FAIL because launch is missing.
- [x] Add launch button, demo banner/library tag, prewritten date label, no refresh controls, and an unanswered-question view with local branch creation. Reuse existing save and navigation functions.
- [x] Run `pnpm check`, `pnpm test:unit`, then `pnpm test:e2e` sequentially before `pnpm build`; expect all PASS.
- [x] Inspect desktop/phone demo rendering and document that provider-free demo interactions differ from offline app loading. Commit.
- [x] Perform one fresh review of this feature, fix meaningful findings with regressions, rebuild/restart the existing private app, and hand over the demo button. Skip all live research calls.
