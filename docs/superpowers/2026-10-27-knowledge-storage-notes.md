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
Question identity includes bounded resolved context. Canonical topics, tree
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
valid/missing/corrupt models, private asset boundaries, SHA/CSP integrity, log
redaction and graceful shutdown. An unconfigured release cannot admit paid work;
the separate real native SIGKILL fixture verifies interruption of pending paid
research and connection-verification stages with zero automatic provider replay.

The first final acceptance pass completed 142 native tests (five explicit
fixture/native gates ignored in the ordinary suite), 67 frontend units, seven
build checks and 26 browser cases. Formatting, strict locked Clippy, Svelte checks,
optimized packaging and packaged recovery verification passed. The explicit native
embedding parity test also passed under restricted networking with an empty PATH.
The standalone 32 MiB import published/read back 1,000 answers atomically in
7.57 seconds, approximately 771 MiB maximum RSS (808,091,648 bytes). `otool -L`
showed only macOS system libraries/frameworks. The scoped Cargo audit passed after
the documented triage; the unfiltered advisory findings remain recorded below.
The required whole-branch review is recorded below once completed. Linux/macOS hosted CI has not been run here; native execution evidence
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
