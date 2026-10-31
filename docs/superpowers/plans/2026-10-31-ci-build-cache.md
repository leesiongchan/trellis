# CI build reuse implementation plan

> **For agentic workers:** Use superpowers:executing-plans for this small workflow change.

**Goal:** Reuse native dependencies and audit tooling while preserving every Linux verification gate.

**Architecture:** A pinned Rust cache action restores dependency artifacts and installed Cargo binaries before verification. Cargo timing artifacts and separate frontend/release steps expose remaining build costs.

**Tech stack:** GitHub Actions, Cargo, pnpm, Ubuntu 26.04.

**Spec:** `docs/superpowers/specs/2026-10-31-ci-build-cache-design.md`.

## Constraints and review focus

- Preserve the two-job cap, pinned toolchain, release profile, and every existing check.
- Configure the actual `api` workspace rather than the repository root.
- Version audit-binary caches explicitly; audit the current lockfiles on cache hits too.
- Save dependency caches only from main, including runs with failed checks; retain compiler timings when a check fails.
- A warm comparison must use the same commit, and include cache transfer/post-job time.

## Task: Cache and measure CI

Modify `.github/workflows/ci.yml` only; application behavior is unchanged.

1. Pin rust-cache v2.9.3 by commit, configure `api -> target`, and version its key for Ubuntu/audit tooling.
2. Separate audit installation and frontend/release verification into observable steps; add Cargo `--timings` and timing-artifact retention.
3. Run actionlint, inspect the diff for preserved gates, and commit/push the workflow and these notes.
4. Verify a complete cold run, successful cache save, then rerun the same commit and verify cache hits and all checks.
5. Compare end-to-end durations and report measured improvement and remaining bottlenecks.

Verification uses actionlint and the real Actions runs; no implementation-mirroring workflow tests are added.
