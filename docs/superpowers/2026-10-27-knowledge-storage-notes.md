# Native knowledge library acceptance — 2026-10-27

Implementation is on `feat/surrealdb-knowledge`, in an isolated worktree. The
original checkout and existing Tailscale routes are unchanged. This record covers
the approved [design](specs/2026-10-27-surrealdb-knowledge-graph-design.md) and
[12-task plan](plans/2026-10-27-surrealdb-knowledge-graph.md).

## Implementation

One Rust process owns embedded SurrealDB 3.3.4/RocksDB with synchronous commits,
an exclusive directory lock and private validated roots. No database listener,
sidecar, Python, container or Node runtime is required. Protocol-2 commands have
durable IDs, payload hashes, revisions and library identity/epoch fences.
Question identity preserves case, Unicode NFC and whitespace normalization, plus bounded resolved context. Case-sensitive units and identifiers are never folded into exact reuse. Canonical topics, tree
appearances and immutable answer versions are separate; refresh changes only the
selected appearance's pin.

Pinned local BGE embeddings produce reversible question/passage candidates using
exact cosine search. Similarity never merges questions or substitutes an answer.
Optional accepted connections retain their verification/model provenance; same
content dismissals survive model replacement. Local indexing and restart recovery
never run paid stages. HNSW remains disabled pending separate measurement.

Browser originals import atomically through bounded JSON staging and remain
unchanged. Separate acknowledged caches support read-only offline access. Saved
navigation, retained knowledge, immutable answer selection, cross-tab conflicts,
explicit restore, export and purge/clear controls are implemented. Demo trees
remain local and opening a fresh demo sends no private-library/provider request.

## Evidence and remaining platform checks

Acceptance logs are retained in the ignored per-plan execution ledger during
implementation. Routine tests use synthetic data, temporary private directories,
fake credentials and local HTTP/HTTPS providers. The release verifier uses the
actual optimized executable from an unrelated directory with an empty child PATH
and no provider credentials. It checks private access/metrics, v1 import, durable
receipts, exclusive locking, SIGKILL/reopen, native backup/restore, fresh epochs,
valid and unavailable model directories, private asset boundaries, SHA/CSP integrity, log
redaction and graceful shutdown. An unconfigured release cannot admit paid work;
the separate real native SIGKILL fixture verifies interruption of pending paid
research and connection-verification stages with zero automatic provider replay.

Final acceptance completed 148 native tests (five explicit fixture/native gates
ignored in the ordinary suite), 76 frontend units, seven build checks and 27
browser cases. Formatting, strict locked Clippy, Svelte checks, optimized packaging
and packaged recovery verification passed. The explicit native embedding parity
test also passed under restricted networking with an empty PATH (5.44 seconds).
The standalone 32 MiB import published/read back 1,000 answers atomically in
7.795 seconds, approximately 776 MiB maximum RSS (813,858,816 bytes), without
compiler/browser contention. `otool -L` showed only macOS system libraries and
frameworks. The scoped Cargo audit passed after the documented four exceptions;
the unfiltered upstream findings remain recorded below. The locked pnpm audit
reported zero vulnerabilities. The whole-branch review and resolutions are below.

Linux/macOS hosted CI has not been run here; native execution evidence
is Apple Silicon macOS. Physical-phone testing, paid live research and deployment
through the actual Tailscale route are unperformed. Phone-sized Chromium and
keyboard checks do not substitute for a physical device.

## Dependency and build-script audit

The locked pnpm audit on 2026-10-27 reports zero vulnerabilities. Installation uses
`--frozen-lockfile --ignore-scripts`; model downloads are explicit, immutable-revision
and SHA-256 checked. The native ONNX archives and hashes are recorded in
`tests/fixtures/library/native-runtime-manifest.json`; the small fastembed CPU
policy patch has its own source digest and patch record. Rust native build scripts
compile RocksDB/bindgen/C++ and the pinned static ONNX runtime. The macOS build
links the compiler's standard availability helper archive discovered through
Clang. Runtime startup downloads nothing.

The unfiltered Cargo.lock audit found four advisories and two maintenance
warnings. These are triaged, not claimed fixed. `.cargo/audit.toml` lists the four
specific exceptions; newly reported advisories still fail the audit. Re-review
these exceptions on every SDK/feature/auth/object-store change:

- [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html) (`rsa`)
  and [RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387.html)
  (`crypto-bigint`): present in the lockfile's optional graph, absent from the
  current native all-features dependency tree. Branch signs its private cookie
  with HMAC-SHA256 and performs no RSA/private-bigint operation.
- [RUSTSEC-2026-0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html) and
  [RUSTSEC-2026-0195](https://rustsec.org/advisories/RUSTSEC-2026-0195.html):
  `quick-xml 0.39.4` is compiled through SurrealDB core → `object_store 0.13.2`'s
  cloud features. The affected attribute/namespace parsers require XML input;
  Branch exposes typed JSON commands and bound repository-owned SQL, not XML,
  object-store/ML imports, arbitrary SQL or remote exports. Native backups use
  local streams. The vulnerable XML path is unreachable in this release's
  application interfaces. Upstream's `^0.39` constraint cannot select the `>=0.41`
  fix without an upstream change; no speculative SDK major upgrade or second
  vendored fork was introduced. This is a reachability assessment, not a guarantee
  about newly enabled SDK functionality. Do not configure SDK object stores for
  this application.
- `atomic-polyfill 1.0.3` and `paste 1.0.15` are upstream maintenance warnings.
  Track removal through upstream dependency releases; no application vulnerability
  was reported for them by this audit.

CI pins Rust 1.98.0, Node 26.5.0 and pnpm 11.21.0, installs native prerequisites,
sets up the checksummed model explicitly, and runs backend/frontend/release gates
on Ubuntu 24.04 and macOS 15. Production browser acceptance and dependency audits
run on Linux. These workflow changes have been reviewed locally; hosted execution
remains unverified.

## Upgrade and recovery

See the [storage runbook](../runbooks/knowledge-storage.md) for exact commands.
Keep the origin, cookie key and private directories stable; deploy the paired
frontend/backend together. Startup rejects incompatible engine/schema markers,
creates a separate pre-upgrade export for compatible release changes, and marks
unfinished paid operations interrupted. The runtime creates a first/daily native
backup with its existing handle and retains seven completed daily snapshots.
Pre-upgrade snapshots are separate. Monitor backup age/failure metrics.

Restore requires a fresh nonexistent target, verifies native export integrity,
counts and versions, and assigns a fresh epoch. Rollback restores the matching
pre-upgrade backup and runs its old release bundle against that fresh directory.
Never downgrade an engine in place. Purging the live library does not delete
browser originals, exported files or retained backups; clear those separately.

## Whole-branch review and resolutions

A fresh-context independent reviewer (`gpt-6-astra`) inspected the complete
`763fe43..7c94386` branch read-only. It found five Important issues. The author
resolved them in one fix pass with focused regressions observed failing first:

1. **Case-sensitive identity and equivalence.** `1 mW`/`1 MW` and `1 MB`/`1 Mb`
   previously shared exact keys, could pin incompatible answers, and could be
   labelled equivalent. Exact keys now preserve case and use key version 2;
   browser sibling comparisons, clarifications and equivalence quantity guards
   preserve these distinctions. Unicode NFC and whitespace reuse remain. This
   deliberately supersedes the plan's unconditional case-fold example.
2. **Unadmitted recovered intent rejection.** A stored intent whose operation
   lookup returned 404 could repeatedly receive a definitive 404/409 on submission
   and block all later writes. Recovery now retires definitive client rejections,
   surfaces their message and permits independent actions. Uncertain, authentication
   and admission-limit outcomes retain their original IDs.
3. **Restore after deletion.** Explicit restore now revalidates completed local
   receipts against the server, restoring a deleted copy while leaving a surviving
   copy idempotent. A real-browser regression deletes an imported exploration,
   restores its original answer/citations on the same browser, repeats restore
   without duplicating it, and observes zero research calls.
4. **Expired staging retries.** An authoritative failed/suppressed upload can
   receive fresh begin, batch and finalization IDs for an explicit retry, carrying
   `explicitRestore: true`. Uncertain staging keeps its IDs. Replayed begin commands
   also run expiry cleanup for staging manifests, so recovery works after 24 hours
   without requiring a process restart.
5. **Immutable answer provenance.** Research versions snapshot the actual server
   input/context and attempt ID. Trusted provider metadata records the requested
   model, separately reported model when supplied, prompt/schema SHA-256, reasoning
   effort, output-token setting and disabled response storage. The provider's outer
   response supplies its reported model; model-generated explanation fields cannot
   supply it. Refreshing with another model preserves the original version's
   metadata. Legacy imports retain original exploration/node IDs and digest;
   older records explicitly report unavailable provenance rather than inventing it.

The native deadline test exposed a harness-only timing issue during verification:
its paused clock could advance while external SDK worker threads initialized.
Setup now uses real time and only the admission deadline uses virtual time, with
one millisecond allowed for Tokio timer-wheel rounding. Production budgets remain
unchanged.

**Deferred minor:** the packaged verifier's invalid-model-directory fixture has
an invalid manifest and missing ONNX artifacts. The loader uses its compiled
pinned manifest, so that case repeats missing-file degradation rather than testing
an actual checksum-invalid ONNX file in the package. The separate native loader
regression does test corrupt ONNX bytes and preserves fail-safe availability.
This package-specific coverage improvement remains deferred; it is not evidence
of a remaining runtime corruption-handling defect.

The reviewer declined to claim hosted CI/Linux linkage, actual tailnet/physical
phone/paid-provider quality, a complete audit of unchanged upstream internals, or
capacity beyond measured fixtures. These limits remain explicit. Review fixes are
verified by their failing-then-passing regressions and the complete suite; no second
reviewer was dispatched.

## Rulings and costs

These decisions preserve the execution ledger's chronological rulings. The final
handoff should be read with their costs and the deferred minor above.

| Tasks | Decision | Cost or limitation |
| --- | --- | --- |
| 1, 3, 4 | Keep raw SQL and commit-control fixtures in private modules. | Verification must include `--lib`; no public test SQL interface exists. |
| 5 | Use `node --test` for build tests, matching the repository. | Both Node and Vitest suites are required. |
| 5 | Vendor the exact fastembed release with an optional safe CPU policy hook. | Maintain/remove the small patch when upstream exposes the needed setting. |
| 7 | Stage UTF-8 JSON fragments rather than requiring one node per record. | Use the shared browser encoder; finalization buffers at most 32 MiB of source JSON. |
| 7 | Preserve legacy 500-scalar questions while new commands use 500 UTF-16 units. | Stored and new-input validators differ deliberately. |
| 7 | Exclude view/revision/transient fields and updatedAt from content identity. | Identical imports preserve the first copy's updatedAt. |
| 7 | Mark imported clarification context unknown instead of assuming context-free identity. | Conservative cross-import/new-topic reuse; legacy content cannot ground accepted factual labels. |
| 7 | Run maximum-import timing/memory checks apart from compiler jobs. | CI retains a separate native maintenance gate. |
| 8 | Validate the actual asset root and reject Git ancestors for private storage. | Fixtures need dedicated roots; operators keep data/backups outside checkouts/assets. |
| 8 | Restore only into a nonexistent target, protected by an in-progress marker. | Operators choose a fresh path; old intents/imports require explicit restore after the fresh epoch. |
| 9 | Use one long shared purge-confirmation constant with offline/backup disclosure. | Frontend and backend must use the same constant. |
| 9 | Require library identity as well as epoch on mutations. | Clients bootstrap before writing, preventing installation collisions at epoch 1. |
| 10 | Bootstrap the private library lazily for a fresh public demo. | New devices open Your explorations to fetch shared knowledge. |
| 10 | Use real native fixture providers for browser success paths. | Fixture minute limits are higher for serial tests; production limits remain separately tested. |
| 10 | Link macOS Clang's standard availability helper archive statically. | macOS builds require that compiler support archive. |
| 10 | Launch compiled native fixtures directly with owned process teardown. | Test/release children use direct signals; generic development groups retain their lifecycle. |
| 12 | Retain the pinned SDK and four explicit reachability-scoped audit exceptions. | Upstream findings remain; re-review on SDK, feature, auth or object-store changes. |
| 12 | Share one pure canonical legacy identity encoder with the release verifier. | Cross-language fixture checks remain important against shared encoder mistakes. |
| 12 | Keep the packaged smoke provider-free; use the native crash fixture for pending paid interruption. | Package-only pending-provider coverage is weaker; no production test/provider bypass was added. |
| 12 | Persist content-pair dismissal receipts and preserve accepted old-model provenance. | Dismissed unchanged content stays suppressed after a model upgrade; fresh versions may suggest again. |
| Final | Use conservative case-sensitive exact key version 2. | Ordinary capitalization can yield separate topics/fewer exact cache hits; old folded aliases are not reused. |
| Final | Store real immutable generation/input provenance and mark unavailable history honestly. | Extra private metadata/storage; reported model remains optional when the provider omits it. |
| Final | Leave hosted CI and Linux linkage unclaimed. | Platform-specific issues may remain until those jobs run. |
| Final | Leave actual Tailscale, physical phone and paid live research unperformed. | Deployment/device/provider-specific issues remain outside local fixture evidence. |
| Final | Review integration and the vendor patch, not every upstream implementation. | Upstream defects can remain outside this independent review. |
| Final | Make no capacity claim beyond the measured fixtures. | Larger libraries may need separately measured search/index changes. |
