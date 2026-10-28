# Obsolete-code cleanup

Removed the unused browser client for the retired /api/research endpoint and the mixed-purpose KnowledgeSession. The local demo now uses DemoSession, which has no research callback, request queue, cancellation machinery or start-research action, and rejects research explorations before replacing local state. Real research continues through the existing server-owned ResearchSession.

Removed the one-off Node-migration fixture generator. Its checked-in golden fixtures remain, since Rust/browser contract and upgrade tests consume them. Removed eight unit tests for the retired controller and added five meaningful demo regressions; all 73 remaining unit tests pass in 12 files. The research-load rejection was observed failing before implementation.

Consolidated browser acceptance into app/playwright.config.ts with all 27 existing production cases. Removed test:e2e:dev and the old normal-development-server harness, which did not supply isolated provider/storage fixtures. Retained scripts/test-server.ts and api/tests/browser_server.rs for the isolated native HTTP/HTTPS/provider/graph test pipeline.

Removed scripts/start.sh; pnpm start directly executes the packaged native binary from the package root. Removed duplicate CI invocation of standalone release verification; test:build already runs the real verifier and checks its results. The standalone verify:release command remains available.

Removed completed ignored database comparison and brainstorm preview harnesses after checking for active processes. Temporary recovery copies are outside the repository at /private/tmp/knowledge-map-obsolete-harnesses-QGmMC1. Existing runtime access settings remain untouched. The active local pnpm store is now ignored.

Verification: Svelte diagnostics, Vite+ formatting/type-aware lint, 73 unit tests, frontend production build, all 7 package/release tests, locked install and CI actionlint/shellcheck passed. Canonical browser discovery still reports 27 cases. Two existing desktop/phone demo browser cases passed through the packaged pnpm start entry using owned temporary native storage and no provider calls; the owned process and data were cleaned up. Static import tracing found no unreferenced frontend modules/components. Independent focused review found no Critical/Important issues. Native source is unchanged; no native rebuild or full 27-case browser execution was repeated. The previously reported low-severity deepmerge audit finding is unchanged.
