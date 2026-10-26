# SurrealDB Knowledge Graph Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist Branch's research library in embedded SurrealDB, connect shared and related knowledge, and preserve existing answers, citations, and browser data.

**Architecture:** One Rust process owns embedded RocksDB-backed SurrealDB and serves typed library commands to Svelte. Topic identity, appearance trees, immutable answers, and reversible connections are separate records. Local embeddings supply candidates; existing provider-backed research remains bounded, grounded, and durable before acknowledgement.

**Tech Stack:** Existing Rust 2024 / Rust 1.98.0, Axum, Tokio, Svelte 5, SvelteKit adapter-static, pnpm 11.21.0, Vitest and Playwright; SurrealDB 3.3.4 with `kv-rocksdb`; fastembed 7.5.0 as the candidate local inference adapter. Pin compatible features/transitives in Cargo.lock and verify native packaging in Tasks 1 and 5. Registry metadata confirmed these two stable crate versions on 2026-10-27; compilation and their combined dependency compatibility are not established by planning.

**Spec:** [Approved SurrealDB design](../specs/2026-10-27-surrealdb-knowledge-graph-design.md). Read both documents before executing. This is one coordinated migration; intermediate commits are not independently deployable releases. Do not deploy, change Tailscale, or call paid providers during routine verification.

## Global Constraints

- One private library for this installation. All devices holding the existing access capability see that same library. There is no per-person isolation or public sharing.
- Embedded SurrealDB inside the existing Rust process, with persistent RocksDB storage. There is no exposed database HTTP/WebSocket listener and no additional runtime database service.
- Exact questions in the same saved context can reuse a topic automatically. Semantic equivalence and relatedness produce reversible connections between separate topics; they do not erase or globally merge existing records.
- Answers have immutable versions. Previously opened appearances remain pinned to the answer they showed until the user chooses an update.
- IndexedDB remains a cache, migration source, and provider-free demo store. SurrealDB becomes the source of truth for research explorations after migration.
- Lists use stable cursors and a maximum page size of 100. Initial graph limits are three hops, 100 nodes, and 200 edges per request.
- Retrieve up to ten candidates per channel and show at most five connections initially. At most three candidates enter one optional provider verification call per explicit research operation. Backfill, startup, and unopened suggestions make no paid calls.
- Use exact cosine retrieval initially. Answer passages have a tokenizer-derived maximum of 350 tokens plus bounded context within the actual model input limit, and at most 64 chunks per answer. Retain full answers and report partial indexing.
- Import limits: 2 MiB per batch, 32 MiB and 2,000 nodes per exploration. Preserve unreadable/oversized originals; never truncate an import.
- Retain operation deduplication records while their exploration exists. Retain content-free deletion/import receipts and reject commands from an earlier library epoch after purge.
- Preserve current research/evidence bounds, private access, the application origin, and port 60090. No public hosting, accounts, offline edit queue, topic consolidation, or full-library graph canvas.
- Preserve the Rust one-process runtime, committed lockfiles, strict Clippy, and no application unsafe code or production unwrap/expect. Tests use owned temporary data directories and fake providers, never the user's database or live service.
- Use the Nix comma fallback for missing commands. Create an isolated worktree through the worktree skill at execution time. Stage only task-owned files and commit after the task's checks pass.

## Review Focus

- Identical wording can mean different things when ancestors, clarification, negation, units, or model versions differ; keys and candidate tests in Tasks 2, 3, and 6 prevent false identity.
- A response can be lost after a durable commit, or a browser can disconnect during research; Task 4 proves replay does not regenerate or lose an acknowledged answer.
- A legacy tab can change IndexedDB after import or reconnect after deletion/purge; Tasks 7 and 10 preserve changed copies without automatically resurrecting deleted content.
- A model can be missing, corrupt, changed, or unable to load its native runtime; Tasks 5 and 8 preserve storage/read availability and expose discovery degradation without downloads at startup.
- Browser caches can belong to another installation or epoch, and disconnected devices cannot be remotely erased; Tasks 9 and 10 enforce authenticated cache identity and honest purge/offline behaviour.

## File structure and shared conventions

Create `backend/src/storage/` for the single datastore handle, schema, typed internal records, atomic operations, topic/appearance mutations, answers, graph queries, imports, recovery, and backups. Use `backend/src/knowledge/` for orchestration over storage, existing jobs/providers, and matching. Keep local inference and chunking in `backend/src/embeddings/`. Database record conversion stays inside storage: current SDK docs require `SurrealValue` for typed values, while HTTP types continue to use Serde. Do not expose SurrealDB record IDs or types over the wire.

Create `backend/src/contracts/library.rs` and `src/lib/knowledge/library-schemas.ts` / `library-types.ts` for matching wire contracts. Keep existing `TopicContent` and schema-v1 validators for research content and import. Split the research session from the demo's local session in `src/lib/knowledge/server-session.svelte.ts`; retain the existing local demo behaviour rather than teaching it server commands.

Put reusable public synthetic cases in `tests/fixtures/library/`, with corresponding Rust integration tests and Vitest fixtures. Port useful assertions from the ignored spike; do not import its scripts or temporary dependencies into production. Native inference assets belong under `release/models/` and required libraries under `release/lib/`, outside `release/public/`.

Public IDs (`LibraryId`, `ExplorationId`, `AppearanceId`, `TopicId`, `AnswerVersionId`, `ConnectionId`, `OperationId`, `ImportId`) are validated opaque string newtypes in Rust and inferred string types in TypeScript. Unless stated otherwise, repository/service methods below are `async`, borrow `&self`, and return `Result<..., AppError>`. The internal datastore handle is private and shared from one open instance.

## Task 1: Prove embedded storage and release build compatibility

**Files:** Create `backend/src/storage/{mod,config}.rs`, `backend/tests/storage.rs`; modify `backend/{Cargo.toml,Cargo.lock}`, `backend/src/lib.rs`; create `docs/runbooks/knowledge-storage.md` with the native build prerequisites verified here.

**Interfaces:** Produce `StorageConfig::from_values(values: &HashMap<String,String>, executable: &Path) -> Result<Self,AppError>`, `KnowledgeStore::open(config: &StorageConfig) -> Result<Self,AppError>`, and `KnowledgeStore::close(&self) -> Result<(),AppError>`. `StorageConfig` contains absolute data/backup paths and an optional model path. Use `APP_DATA_DIR` explicitly; default backup location to a separate sibling `backups` directory and model location to the executable's sibling `models` directory. Reject overlapping data, backup, public-asset, and model roots.

- [x] **Step 1 — Write failing integration checks.** `embedded_round_trip_and_exclusive_open` writes a nested answer to an owned temporary RocksDB directory, closes/reopens it, and asserts exact source/paragraph preservation; a simultaneous second open must fail. `invalid_storage_roots_are_rejected` covers relative paths, symlink escape, and backup/data overlap. These use a real embedded datastore, not HTTP or mocks.
Assertion anchors (fixture variables are local to these named tests):

```rust
assert_eq!(reopened_answer, original_answer);
assert!(second_open.is_err());
```

- [x] **Step 2 — Verify red.** Run `cargo test --manifest-path backend/Cargo.toml --test storage`; expect missing storage interfaces or failed persistence assertions, not an unexplained compiler/toolchain failure.
- [x] **Step 3 — Implement the foundation.** Pin SurrealDB 3.3.4 with default features disabled and only required local features. Set the SDK channel capacity to 32 and bound datastore admission to 16 concurrent calls with a five-second deadline. Open one fixed namespace/database, configure synchronous durability through the documented embedded API, create restricted directories, and close through the supported API. Keep schema installation for Task 2. Inspect native build scripts/dependency provenance and record C++/libclang requirements; preserve existing toolchain versions.
- [x] **Step 4 — Verify the gate.** Run the storage test with `--locked`, `cargo build --manifest-path backend/Cargo.toml --release --locked`, strict Clippy, and rustfmt. Inspect native linkage using platform tools (`otool -L` on macOS; `ldd` on Linux). Record required runtime libraries rather than assuming they are bundled. A missing compatible feature/API or one-process deployment failure stops later tasks for a concrete design revision.
- [x] **Step 5 — Commit.** `feat: add embedded SurrealDB storage foundation`.

## Task 2: Define library contracts, schema, and operation replay

**Files:** Create `backend/src/contracts/library.rs`, `backend/src/storage/{schema,records,operations}.rs`, `backend/tests/{library_contracts,operations}.rs`, `src/lib/knowledge/{library-schemas,library-types}.ts`, `tests/unit/library-contracts.test.ts`, and `tests/fixtures/library/contracts.json`; modify storage/contracts exports and `backend/src/error.rs`.

**Interfaces:** Produce `Command<T> { operation_id, library_epoch, expected_revision: Option<u64>, payload: T }`; wire fields are camelCase. Define the input types used later: `CreateExploration { question, locale }`, `AddBranch { parent_appearance_id, question }`, `ResearchInput { appearance_id, refresh, clarification }`, `ChooseAnswer { appearance_id, answer_version_id }`, `DismissConnection { connection_id }`, `DeleteExploration { exploration_id }`, and `PurgeLibrary { confirmation }`. Define `LibraryInfo`, `ExplorationSummary`, `ExplorationSnapshot`, `TopicView`, `ConnectionView`, `Page<T>`, `OperationView`, `OperationResult`, and migration types consumed in Task 7. `OperationView.state` is pending/completed/failed/interrupted; results are a tagged union for exploration, research, connection, deletion, purge, and import outcomes.

Produce `KnowledgeStore::install_schema() -> Result<(),AppError>`, `claim_operation(intent: &OperationIntent) -> Result<ClaimOutcome,AppError>`, `operation(id: &OperationId) -> Result<OperationView,AppError>`, and `question_key(input: &ResolvedQuestion) -> QuestionKey`. `OperationIntent` contains command ID, epoch, stable canonical payload hash, target, and creation time; `ClaimOutcome` is claimed/replay. `ResolvedQuestion` contains normalized wording, nullable clarification, locale, bounded stored ancestors, interpretation, and normalization/context encoding version.

- [x] **Step 1 — Write failing checks.** Shared Rust/TS fixtures assert required nullable fields, opaque IDs, protocol/schema version 2, existing content limits, and unknown-field handling. `exact_key_preserves_context` asserts different keys for “volatile”/“nonvolatile,” units, changed ancestors/clarification/locale, and stable keys for agreed whitespace/case normalization. `operation_claim_is_atomic` sends 12 simultaneous identical intents: exactly one is claimed, the rest replay; a reused ID with a different payload or old epoch returns conflict.
Assertion anchors (fixture variables are local to these named tests):

```rust
assert_eq!(claimed, 1);
assert_eq!(replayed, 11);
assert_ne!(volatile_key, nonvolatile_key);
```

- [x] **Step 2 — Verify red.** Run `cargo test --manifest-path backend/Cargo.toml --test library_contracts --test operations` and `pnpm exec vitest run tests/unit/library-contracts.test.ts`; failures must identify missing contracts or invariants.
- [x] **Step 3 — Implement types and schema version 1.** Install explicit record/relationship schemas and unique indexes for exact keys, operation IDs, one active attempt per topic, appearance ownership, answer revisions, directional edges, symmetric pairs, and import digests. Keep mutable topic pointers separate from immutable answers. Add safe error codes for storage unavailable/busy, revision conflict, intent conflict, interrupted work, invalid import, and reload required. Retry only serialization conflicts, at most three attempts; never run an external effect inside this retry.
- [x] **Step 4 — Verify green.** Re-run both targeted commands with Cargo `--locked`, then the existing contract suite and `pnpm check`. Confirm repository records and wire types do not leak SDK types into Svelte.
- [x] **Step 5 — Commit.** `feat: define library contracts and atomic operation replay`.

## Task 3: Save topic appearances and preserve shared content on deletion

**Files:** Create `backend/src/storage/library.rs` and `backend/tests/library.rs`; extend `storage/records.rs` and `operations.rs`.

**Interfaces:** Produce `create_exploration(command: &Command<CreateExploration>) -> OperationView`, `add_branch(command: &Command<AddBranch>) -> OperationView`, `exploration(id: &ExplorationId) -> ExplorationSnapshot`, `explorations(cursor: Option<&str>, limit: usize) -> Page<ExplorationSummary>`, `topic(id: &TopicId, version: Option<&AnswerVersionId>) -> TopicView`, `delete_exploration(command: &Command<DeleteExploration>) -> OperationView`, and `purge_library(command: &Command<PurgeLibrary>) -> OperationView`. Define `ExplorationSummary` in Task 2's library contract module. Snapshots contain appearance IDs/parentage, topic references, original questions/context, answer pins, and a content revision; view state is excluded.

- [x] **Step 1 — Write failing checks.** `shared_topic_has_distinct_appearances` creates two explorations with identical resolved parent context reaching the same scoped question and asserts one topic, two appearances, original branch questions retained, and a valid tree in each snapshot. `different_context_is_not_identity` preserves identical “What is Apple?” questions under fruit and computing contexts. `delete_preserves_library_and_purge_fences_replay` checks shared topic retention, support-counted branch edges, deletion tombstones, changed epoch after purge, and rejection of old intents without recreating text. A lost purge response can replay its own content-free receipt and learn the new epoch without purging again. Include stale-revision and endpoint-ownership failures.
Assertion anchors (fixture variables are local to these named tests):

```rust
assert_eq!(first_appearance.topic_id, second_appearance.topic_id);
assert_ne!(first_appearance.id, second_appearance.id);
assert!(epoch_after_purge > epoch_before_purge);
```

- [x] **Step 2 — Verify red.** Run `cargo test --manifest-path backend/Cargo.toml --test library` and inspect the failed semantic assertions.
- [x] **Step 3 — Implement atomic mutations.** Resolve exact identity through unique question keys and reread after races. Create/update exploration, appearances, occurrence support, and operation result in one transaction. Scope all references to the library and correct exploration; suppress canonical self-edges. Use stable list ordering by timestamp and ID. Exploration deletion retains topics/versions; explicit purge removes content and derived records while retaining content-free receipts and advancing epoch.
- [x] **Step 4 — Verify green.** Run library, operations, and storage integration tests `--locked`; assert a failed mutation leaves counts, pins, and revisions unchanged. Check page-size values 1, 100, and 101 with the documented maximum.
- [x] **Step 5 — Commit.** `feat: persist shared topics and exploration appearances`.

## Task 4: Commit researched answers before acknowledging completion

**Files:** Create `backend/src/knowledge/{mod,service,research}.rs`, `backend/src/storage/answers.rs`, and `backend/tests/saved_research.rs`; modify `backend/src/jobs/mod.rs`, `backend/tests/jobs.rs`, and library/storage exports.

**Interfaces:** Produce `KnowledgeService::new(store: Arc<KnowledgeStore>, jobs: Arc<Coordinator>, provider: Arc<dyn ResearchProvider>) -> Self`, `KnowledgeService::start_research(command: Command<ResearchInput>) -> Result<OperationView,AppError>`, `KnowledgeStore::choose_answer(command: &Command<ChooseAnswer>) -> OperationView`, and `KnowledgeService::drain() -> Result<(),AppError>`. Add `Coordinator::admit_work(self: &Arc<Self>, key: JobKey, work: JobWork) -> Result<Waiter,AppError>`; `JobKey` is a server-created attempt key and `JobWork` is a boxed Send one-shot closure from `JobContext` to `ProviderFuture<'static,ResearchResult>`. Preserve existing `admit(ResearchRequest)` as a wrapper for current coordinator tests; the new library runner owns its waiter until completion independently of HTTP polling.

- [x] **Step 1 — Write failing checks.** `commit_precedes_completion` pauses a database commit and asserts the operation remains pending. `lost_response_replays_without_research` completes, drops the original caller, replays the ID, and asserts one provider sequence and identical pinned version. `refresh_keeps_other_pins` asserts a new version only repins the selected appearance. Race 12 refreshes for one topic and assert one admitted paid attempt. Cover disconnect, stale generation completion after purge/delete, clarification, grounding failure, and unknown upstream outcome without automatic retry.
Assertion anchors (fixture variables are local to these named tests):

```rust
assert_eq!(search_calls, 1);
assert_eq!(generation_calls, 1);
assert_eq!(other_appearance_pin_after_refresh, other_appearance_pin_before_refresh);
```

- [x] **Step 2 — Verify red.** Run `cargo test --manifest-path backend/Cargo.toml --test saved_research --test jobs`; provider fixtures must count invocations, not depend on paid services.
- [x] **Step 3 — Implement the durable runner.** Claim intent/active topic attempt before calling out; derive all ancestors from stored appearances. Wrap existing `research_topic` in admitted work. Atomically commit validated content, immutable sources/version, selected pin, at most six suggested appearances, revision, and operation result. Resolve empty/suggested branches by exact keys; no recursive provider calls. Hold admission through the optional post-commit matching hook introduced in Task 6, within the original deadline. Mark storage failures/interrupted attempts safely; a persisted successful answer cannot be demoted by matching failure.
- [x] **Step 4 — Verify green.** Re-run saved-research, jobs, research, providers, operations, and library tests `--locked`. Assert earlier answers survive every failure and choosing an incompatible version is rejected. Provider retries remain explicit new user intents.
- [x] **Step 5 — Commit.** `feat: persist research results and immutable answer versions`.

## Task 5: Integrate local embeddings with a degraded discovery mode

**Files:** Create `backend/src/embeddings/{mod,model,chunking,worker}.rs`, `backend/src/storage/embeddings.rs`, `backend/tests/embeddings.rs`, `scripts/fetch-embedding-model.ts`, `tests/fixtures/library/embedding-cases.json`, and `tests/fixtures/library/model-manifest.json`; modify Cargo manifests and `scripts/package-release.ts` / its build tests.

**Interfaces:** Define `Embedder: Send + Sync` with `embed(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>,EmbeddingError>`, `EmbeddingInput { text, kind: Query|Document }`, and `ModelManifest { model_id, revision, dimensions, max_tokens, artifact_hashes, preprocessing_version }`. Produce `LocalEmbedder::load(root: &Path, manifest: &ModelManifest) -> Result<Self,EmbeddingError>`, `chunk_answer(content: &TopicContent, manifest: &ModelManifest) -> Result<ChunkSet,AppError>`, `EmbeddingWorker::start(store: Arc<KnowledgeStore>, embedder: Arc<dyn Embedder>, cancel: CancellationToken) -> EmbeddingWorker`, `EmbeddingWorker::shutdown(&self) -> Result<(),AppError>`, and `KnowledgeStore::nearest(input: &SearchVector, filter: &SearchFilter, limit: usize) -> Vec<SearchHit>`. Define ChunkSet with paragraph locations and partial-coverage flag; SearchVector includes model version, SearchFilter restricts channel and current/pinned answer visibility, and SearchHit identifies source/topic/version, passage location and score.

- [x] **Step 1 — Write failing checks.** `missing_or_corrupt_model_does_not_break_saves` checks discovery degraded with functioning storage. `chunking_preserves_locations_and_limits` checks 350-token passages, 64-chunk ceiling, heading/context input within tokenizer limit, and partial coverage. `model_versions_do_not_mix` rejects wrong dimensions/non-finite vectors and excludes other model generations. Commit small vectors and known ranking cases from the public spike fixture rather than private text.
Assertion anchors (fixture variables are local to these named tests):

```rust
assert!(chunks.len() <= 64);
assert_eq!(vector.len(), 384);
assert!(input_tokens <= manifest.max_tokens);
```

- [x] **Step 2 — Verify red.** Run `cargo test --manifest-path backend/Cargo.toml --test embeddings` plus `pnpm exec vitest run tests/build/package-release.test.ts` where packaging changes need coverage.
- [x] **Step 3 — Implement inference and derived indexing.** Pin fastembed 7.5.0 and its compatible ONNX runtime; disable automatic network retrieval at runtime. Pin the spike's quantized model `Qdrant/bge-small-en-v1.5-onnx-Q`, revision `aa8f8b060edb00e03bfdd08813a2949946c8ba55`, tokenizer, and file hashes in the model manifest before fetching (384 dimensions). The explicit build/setup script downloads only those pinned files and verifies hashes. Run inference on a dedicated blocking worker with one active batch, at most 32 inputs, and two inference threads. Persist one resumable indexing state per source/model/input hash; update it transactionally and leave user saves successful when discovery is unavailable.
- [x] **Step 4 — Verify green and the packaging gate.** Run embedding tests `--locked` with fake inference, then one explicit local-model integration check against the pinned public fixture. Run it with network disabled and with PATH excluding Python/Node/database CLIs. Package necessary native libraries under `release/lib/` with executable-relative loading. Record compatible dependencies/artifact hashes and test query/document preprocessing parity. Do not replace the embedding provider or require a Python process if native integration fails.
- [x] **Step 5 — Commit.** `feat: add bounded local embedding and exact retrieval`.

## Task 6: Connect equivalent questions and overlapping explanations

**Files:** Create `backend/src/knowledge/{matching,verification}.rs`, `backend/src/storage/connections.rs`, `backend/src/providers/schemas/connections.json`, `backend/tests/{connections,matching}.rs`, `tests/fixtures/library/matching.json`; modify knowledge service, provider interfaces/adapters/fixtures, and telemetry stage definitions.

**Interfaces:** Produce `MatchingService::new(store: Arc<KnowledgeStore>, embedder: Arc<dyn Embedder>) -> Self`, `MatchingService::discover(source: &MatchSource) -> Result<MatchSummary,AppError>`, `KnowledgeStore::connections(topic: &TopicId, cursor: Option<&str>, limit: usize) -> Page<ConnectionView>`, `neighborhood(topic: &TopicId, bounds: GraphBounds) -> GraphNeighborhood`, and `dismiss_connection(command: &Command<DismissConnection>) -> OperationView`. MatchSource identifies a topic/question variant or immutable answer version; MatchSummary counts candidates and links. GraphBounds contains hops/node/edge limits and GraphNeighborhood contains bounded nodes/edges plus truncation metadata. Add `ConnectionVerifier::verify(candidates: &[MatchCandidate], ctx: &JobContext) -> ProviderFuture<ConnectionVerdict>` through the existing configured provider; verdict choices are equivalent/related/different/uncertain with bounded reasons and referenced candidate IDs. Add `KnowledgeService::with_matching(self, matching: Arc<MatchingService>, verifier: Arc<dyn ConnectionVerifier>) -> Self` to attach the post-commit hook before sharing the service. Keep this boundary separate from evidence grounding.

- [ ] **Step 1 — Write failing checks.** `matching_preserves_originals` asserts equivalent paraphrases create a symmetric link without changing either topic/version; related/negated/unit/context contrasts never become identity. `passage_overlap_records_both_versions` checks paragraph IDs and immutable source references. `cycle_walk_respects_bounds` checks three hops, 100 nodes, 200 edges and query deadline. `dismissed_pairs_stay_dismissed` covers duplicate discovery and new-content versions. Fixture cases must include related-but-different flash questions, Apple contexts, negation, and irrelevant nearest neighbours; count zero provider calls for backfill and unopened suggestions.
Assertion anchors (fixture variables are local to these named tests):

```rust
assert_eq!(accepted_equivalence_edges_for_hard_negatives, 0);
assert!(walk.nodes.len() <= 100);
assert!(walk.edges.len() <= 200);
```

- [ ] **Step 2 — Verify red.** Run `cargo test --manifest-path backend/Cargo.toml --test connections --test matching`.
- [ ] **Step 3 — Implement retrieval and links.** Retrieve at most ten per question/question, question/passage, and passage/passage channel; combine keyword/vector ranks, collapse candidates, and return at most five with provenance. Use exact cosine and preserve all originals. Vector hits begin as labelled similarity candidates, without a universal acceptance cutoff. In the explicit research post-commit hook only, persist a stage intent and allow one verifier call for at most three candidates using remaining original job time; uncertainty/failure leaves candidates. Equivalent verdicts can create reversible equivalence edges; factual relationship labels require existing cited support. No restart/backfill paid verification.
- [ ] **Step 4 — Verify green.** Run connections, matching, saved-research, providers, and embeddings tests `--locked`. A query with only unrelated neighbours must produce no accepted equivalence edge. A verifier inventing candidate IDs, citing absent paragraphs, or trying to alter a record must fail closed. Record per-case outcomes; all hard negative equivalence cases must pass before exposing the equivalence label.
- [ ] **Step 5 — Commit.** `feat: connect related questions and answer passages`.

## Task 7: Import browser explorations without overwriting their originals

**Files:** Create `backend/src/storage/imports.rs`, `backend/src/contracts/legacy.rs`, `backend/tests/migration.rs`, and `tests/fixtures/library/migration.json`; extend Task 2 migration wire contracts.

**Interfaces:** Define `BeginImport { legacy_exploration_id, content_digest, expected_bytes, expected_nodes }`, `ImportBatch { import_id, batch_index, records }`, `FinalizeImport { import_id }`, and `ImportView { state, mapping, server_revision, error }`. Produce `begin_import(command: &Command<BeginImport>) -> ImportView`, `stage_import(command: &Command<ImportBatch>) -> ImportView`, `finalize_import(command: &Command<FinalizeImport>) -> ImportView`, and `import_status(id: &ImportId) -> ImportView`. `LegacyExploration::validate(Value)` matches existing schema-v1 semantics; digest encoding excludes local view/revision and is versioned/shared with TypeScript.

- [ ] **Step 1 — Write failing checks.** `import_is_atomic_and_idempotent` compares all questions, parentage, answer/source snapshots, timestamps and mapped IDs; an interrupted staged import is invisible. `changed_legacy_copy_is_preserved` creates a distinct copy instead of overwriting its earlier server import. Test identical copies from two origins, demo exclusion, invalid references, duplicate batch indices, failed finalization, old-epoch receipts, and boundary cases of 2 MiB, 32 MiB, and 2,000 nodes plus one unit over each limit. Generate large cases in tests, not committed fixture blobs.
Assertion anchors (fixture variables are local to these named tests):

```rust
assert_eq!(visible_records_before_finalize, 0);
assert_eq!(repeated_import.mapping, first_import.mapping);
```

- [ ] **Step 2 — Verify red.** Run `cargo test --manifest-path backend/Cargo.toml --test migration`.
- [ ] **Step 3 — Implement staged publication.** Bound staging reads, two active imports, and staging lifetime to 24 hours; cleanup may delete only unpublished staging records. Validate full tree/content/digest before a bounded finalization transaction publishes mapped records and the receipt. Preserve distinct legacy answers even where an exact question key reuses a topic. Use digest receipts across devices; deleted/purged receipts suppress automatic resurrection. Oversized/unreadable cases return actionable export guidance without discarding input.
- [ ] **Step 4 — Verify green.** Run migration, library, operations, and library-contract tests `--locked`; exercise a 32 MiB finalization on the actual embedded engine within a 30-second maintenance deadline and record peak memory. Any engine limit preventing atomic publication requires reducing the transaction's internal batch strategy while retaining a single visibility fence, not partial publication.
- [ ] **Step 5 — Commit.** `feat: migrate legacy exploration snapshots safely`.

## Task 8: Add startup recovery, native backups, and storage lifecycle

**Files:** Create `backend/src/storage/{recovery,backups}.rs`, `backend/src/knowledge/lifecycle.rs`, `backend/tests/{storage_lifecycle,backups}.rs`; modify `backend/src/{main,config,app,server,telemetry}.rs`, `backend/tests/{lifecycle,common/mod,browser_server}.rs`, `.env.example`, and the storage runbook.

**Interfaces:** Produce `KnowledgeRuntime::start(config: Arc<Config>, store: Arc<KnowledgeStore>, service: Arc<KnowledgeService>, worker: Option<EmbeddingWorker>, cancel: CancellationToken) -> Result<Self,AppError>`, `KnowledgeRuntime::shutdown(&self) -> Result<(),AppError>`, `KnowledgeStore::recover_interrupted() -> RecoverySummary`, `KnowledgeStore::backup(destination: &Path) -> BackupManifest`, and `KnowledgeStore::restore(source: &Path, destination: &Path) -> BackupManifest`. RecoverySummary counts interrupted attempts and resumed derived work; BackupManifest records schema/engine/model versions, checksums, counts, and source-library epoch. Add executable subcommands `storage backup --output PATH` and `storage restore --input PATH --data-dir PATH`; restore requires a fresh empty destination and exclusive datastore access. `Config` now owns StorageConfig; every test/fixture process receives an owned temporary APP_DATA_DIR.

- [ ] **Step 1 — Write failing checks.** A child process acknowledges a save, receives SIGKILL, restarts, and retains nested content/pins; unfinished research and paid verification become interrupted with zero automatic provider calls. Backup restores graph/version/source counts and representative retrieval into a new directory. Test wrong-version restore, checksum tampering, symlink/overlapping destinations, second-process lock, missing model with ready storage, migration failure, and the pre-upgrade backup rollback path.
Assertion anchors (fixture variables are local to these named tests):

```rust
assert_eq!(restored_counts, original_counts);
assert_eq!(automatic_provider_calls_after_restart, 0);
```

- [ ] **Step 2 — Verify red.** Run `cargo test --manifest-path backend/Cargo.toml --test storage_lifecycle --test backups --test lifecycle`.
- [ ] **Step 3 — Implement lifecycle and operational state.** Open/install schema before library readiness; fail clearly for bad storage, but degrade only discovery for bad model assets. Resume derived indexing, never paid stages. Use a maintenance lease for consistent SDK export: pause new writes, refuse if paid work is active, pause derived workers, and keep reads available. Write backup files atomically with hashes. Schedule a daily backup only while the app runs, also before upgrades; keep seven completed daily backups and a separate pre-upgrade backup. The first eligible start creates the first backup; skip/defer busy periods and expose overdue/failure state. The running service uses its existing datastore handle for scheduled exports; the standalone backup CLI requires the service stopped and reports a lock error instead of opening a second writer. Restore into a fresh directory and verify before declaring success; never copy an open RocksDB directory as the backup.
- [ ] **Step 4 — Verify green.** Run all targeted lifecycle/backup checks `--locked`, inspect explicit synchronous durability setup, and verify shutdown stays within the existing 125-second grace / 130-second total with no transaction held over inference or HTTP. Record library/storage/discovery readiness and backup age safely in existing telemetry. No data text/vectors/paths/secrets in logs or metric labels.
- [ ] **Step 5 — Commit.** `feat: add knowledge storage recovery and verified backups`.

## Task 9: Expose authenticated library commands and cut over research

**Files:** Create `backend/src/http/library.rs` and `backend/tests/library_http.rs`; modify `backend/src/{app,error}.rs`, `http/{mod,guard,research,health}.rs`, related existing HTTP tests, and shared API fixtures.

**Interfaces:** Register the routes below, always using Task 2 wire types and `X-Branch-Protocol: 2` for library endpoints. New mutations accept Command envelopes. Immediate durable mutations return completed OperationView; admitted research returns 202 with a Location for polling; replay returns the stored state without another effect.

| Route | Handler result |
| --- | --- |
| `GET /api/library` | LibraryInfo including library ID, epoch and discovery status |
| `GET/POST /api/explorations` | Page of summaries / create operation |
| `GET/DELETE /api/explorations/{id}` | Snapshot / revision-guarded deletion operation |
| `POST /api/explorations/{id}/appearances` | Add-branch operation |
| `POST /api/appearances/{id}/research` | Saved research/refresh operation |
| `PATCH /api/appearances/{id}/answer` | Choose compatible saved version |
| `GET /api/topics/{id}` | TopicView, optional answerVersionId |
| `GET /api/topics/{id}/connections` | Paginated connections and saved locations |
| `GET /api/topics/{id}/neighborhood` | Bounded graph neighbourhood |
| `PATCH /api/connections/{id}` | Dismiss a candidate |
| `GET /api/operations/{id}` | OperationView |
| `POST /api/imports`, `POST /api/imports/{id}/batches`, `POST /api/imports/{id}/completion`, `GET /api/imports/{id}` | Migration manifest/staging/publication |
| `POST /api/library/purge` | Confirmed purge and new epoch |

- [ ] **Step 1 — Write failing socket tests.** `library_requires_access_on_reads_and_writes` covers absent/expired cookies and direct topic/version/operation reads. Mutations reject bad/missing Origin and Host, stale protocol/epoch, foreign appearance IDs, mixed route/payload targets, oversize/chunked bodies and injected query-looking text. `research_replay_after_disconnect` polls the same admitted attempt to durable completion. `legacy_research_requires_reload` asserts the old endpoint returns safe reload guidance with zero provider calls. Read-only saved access works with providers unconfigured.
Assertion anchors (fixture variables are local to these named tests):

```rust
assert_eq!(anonymous_read.status(), StatusCode::UNAUTHORIZED);
assert_eq!(stale_revision.status(), StatusCode::CONFLICT);
assert_eq!(legacy_provider_calls, 0);
```

- [ ] **Step 2 — Verify red.** Run `cargo test --manifest-path backend/Cargo.toml --test library_http --test http --test access`.
- [ ] **Step 3 — Implement the boundary.** Use the existing auth/origin/header/error patterns and no-store responses. Keep normal JSON requests at 65,536 bytes with ten-second reads; only import batches receive their explicit 2 MiB bound. Reject unsupported cursors/limits safely; graph requests have a two-second datastore query deadline in addition to depth/count limits. Purge requires a fresh epoch plus the exact confirmation phrase `Delete my saved library`; it cannot erase another installation's data or bypass the maintenance gate. Keep the static shell/demo public. Treat the old research handler as reload-required after this cutover.
- [ ] **Step 4 — Verify green.** Run library HTTP, access, HTTP, header/static-file, and shared-contract tests `--locked`. Exercise actual timeout/framing failures and verify safe status/envelopes, including 409 conflict, 401 access required, 413 too large, and 503 storage unavailable. Confirm no general SQL/database/file endpoint exists.
- [ ] **Step 5 — Commit.** `feat: expose private durable library APIs`.

## Task 10: Move research sessions to server commands and migrate browser data

**Files:** Create `src/lib/knowledge/{library-client,cache,migration}.ts`, `server-session.svelte.ts`, `tests/unit/{library-client,cache,migration,server-session}.test.ts`; modify `src/routes/+page.svelte`, `src/lib/knowledge/{persistence,types,tree}.ts`, `src/lib/components/{Workspace,TopicTree,TreeCanvas}.svelte`, `scripts/{test-server,lib/process}.ts`, `backend/tests/browser_server.rs`, and browser persistence/recovery/upgrade fixtures.

**Interfaces:** Produce `createLibraryClient(fetcher: typeof fetch): LibraryClient` with typed methods matching Task 9, `createResearchSession(options: {client: LibraryClient; cache: LibraryCache; onChange: (value: WorkspaceExploration) => void}): ResearchSession`, `createLibraryCache(): LibraryCache`, and `migrateLegacy(repository: ExplorationRepository, client: LibraryClient, cache: LibraryCache): Promise<MigrationSummary>`. Define WorkspaceExploration/WorkspaceSession in the common types module as the rendering fields/actions used by the existing tree and reader; demo and research project into that interface without pretending a server snapshot is schema v1. ResearchSession exposes start/select/expand/collapse/addQuestion/refresh/clarify/setView, but content actions send commands and view changes stay local. MigrationSummary counts imported/skipped/failed copies and maps legacy IDs. Never autosave research snapshots back to the server. Cache keys include library ID, epoch, exploration ID, and protocol version.

- [ ] **Step 1 — Write failing Vitest checks.** `retry_preserves_intent_id` drops a response and asserts one ID/payload on replay; pending operations are resolved before new writes. `migration_keeps_v1_original` preserves the original store byte-for-byte and its node IDs through mappings. `legacy_edit_after_import_creates_copy` detects changed digests. `purged_or_other_library_cache_is_not_reused` invalidates mismatched identity/epoch without deleting the migration original silently. `offline_cache_is_read_only` shows saved content while mutations are disabled and no local edit queue appears. Demo tests still assert zero library/provider requests.
Assertion anchors (fixture variables are local to these named tests):

```typescript
expect(retry.operationId).toBe(first.operationId);
expect(originalV1After).toEqual(originalV1Before);
expect(offlineMutationRequests).toHaveLength(0);
```

- [ ] **Step 2 — Verify red.** Run `pnpm exec vitest run tests/unit/library-client.test.ts tests/unit/cache.test.ts tests/unit/migration.test.ts tests/unit/server-session.test.ts`.
- [ ] **Step 3 — Implement clients/cache/migration/session.** Persist intent IDs before submission, poll pending operations with bounded backoff from 250 ms to two seconds, and retain pending state across reload. Stop polling after terminal state or disposal. Validate snapshots/contracts, reject stale responses by session generation, preserve local view state, and use explicit reload on revision conflict. Migrate in bounded batches only after authentication; label progress/refusals and acknowledge server mapping separately from untouched v1 data. Cache only server-acknowledged content. On reconnect, verify library identity/epoch/revision before showing cache as current; unavailable servers leave it visibly offline. Other devices' caches clear only when they reconnect; explain local clearing for disconnected copies.
- [ ] **Step 4 — Verify green.** Run the new and existing unit suites plus `pnpm check`. Update the browser harness to real disposable embedded stores and local fake provider responses; routine UI research cases must exercise server persistence rather than mock successful answer delivery around it. Run production persistence, recovery, upgrade, and demo browser scenarios against the packaged app.
- [ ] **Step 5 — Commit.** `feat: use server library sessions and preserve legacy browser data`.

## Task 11: Expose saved connections, version choice, and library deletion

**Files:** Create `src/lib/components/{TopicConnections,AnswerVersions,LibraryStatus,RetainedTopic}.svelte`, `tests/e2e/library-graph.spec.ts`; modify `components/{Workspace,TopicReader,ExplorationList,WorkspaceControls}.svelte`, `routes/+page.svelte`, and `src/app.css`.

**Interfaces:** `TopicConnections` consumes `Page<ConnectionView>` and callbacks `onFollow(connectionId)` / `onDismiss(connectionId)`; `AnswerVersions` consumes the selected TopicView and `onChoose(answerVersionId)`; `RetainedTopic` renders a TopicView without an appearance and offers `onStartExploration(topicId)`. Use Task 10 session/client methods, not direct fetches from each component. The root page owns return-location history across explorations.

- [ ] **Step 1 — Write failing browser checks.** Two authenticated browser contexts open the same server library. A connection opens an existing appearance in another exploration and returns to the original position without research; an orphan topic opens read-only and can seed a new exploration. Assert similarity/equivalence/answer-overlap labels and reasons, five-item initial connections with pagination, dismiss/reload persistence, older answer citations after another appearance refreshes, explicit version choice, exploration-delete retention copy, and confirmed purge/epoch cache invalidation. Exercise keyboard controls, narrow viewport navigation, and literal rendering of hostile source/model text.
Assertion anchors (fixture variables are local to these named tests):

```typescript
expect(researchRequestsAfterFollow).toBe(researchRequestsBeforeFollow);
expect(originalAnswerSourcesAfterRefresh).toEqual(originalAnswerSourcesBeforeRefresh);
```

- [ ] **Step 2 — Verify red.** Run `pnpm exec playwright test --config playwright.production.config.ts tests/e2e/library-graph.spec.ts` against the disposable fixture server; verify missing interactions rather than weakening assertions to current UI.
- [ ] **Step 3 — Implement these interactions.** Retain the existing tree/reader workspace and mobile Map/Read switch. Show selected-topic connections and newer-version availability; do not invent a full-library graph screen. Starting from a retained topic sends its saved question through ordinary CreateExploration: the new root context still follows exact-reuse rules and does not silently substitute an incompatible old answer. Use accessible buttons/status text and explicit loading/error states. Distinguish deleting an exploration from purging retained library knowledge; explain offline browser originals and backup retention at purge. A missing model is a discovery status, never “no related topics exist.”
- [ ] **Step 4 — Verify green.** Run `pnpm check`, relevant units, the targeted browser file, and existing knowledge-tree/mobile/demo suites. Inspect desktop/mobile behaviour through the browser skill, with no private production data. Assert cross-device reads and both current/pinned answer citations against real fixture storage.
- [ ] **Step 5 — Commit.** `feat: navigate topic connections and saved answer history`.

## Task 12: Verify the packaged release and document upgrade recovery

**Files:** Modify `scripts/{package-release,verify-release,dev}.ts`, `tests/build/{package-release,verify-release}.test.ts`, `.github/workflows/ci.yml`, `README.md`, `.env.example`, and `docs/runbooks/{knowledge-storage,research,rust-service}.md`; create `docs/superpowers/2026-10-27-knowledge-storage-notes.md` recording actual checks and any remaining limitations.

**Interfaces:** Release packaging includes checksummed model/native-library manifests outside public assets; `verify-release.ts` supplies isolated absolute data/backup/model roots and exercises the real binary. The existing commands remain the supported entry points. CI uses the same pinned library/model manifests and separate macOS/Linux native builds.

- [ ] **Step 1 — Write failing release checks.** Launch from an unrelated directory with empty PATH and no providers: create/read library content, detect missing versus valid model assets, reject a second writer, and close/reopen storage. Keep models/native libraries inaccessible through static routes. Verify a synthetic v1 upgrade through the unchanged origin/cookie, an interrupted operation after restart, a restored backup, and no Node/Python/container requirement. Assert failed model checks preserve saved-library availability.
Assertion anchors (fixture variables are local to these named tests):

```typescript
assert.equal(await closed, 0);
assert.equal(paidProviderRequests, 0);
```

- [ ] **Step 2 — Verify red.** Run the new build/release tests against the current packaging and record the missing behaviours; no real APP_DATA_DIR or forwarding listener is used.
- [ ] **Step 3 — Complete packaging, CI, and runbooks.** Pin build prerequisites and downloaded artifact hashes, audit locked dependencies/scripts, and preserve asset integrity/CSP checks. Document explicit model setup, data/backup roots, stale-tab reload, same-key shared access, migration originals/oversized export, daily/pre-upgrade backup status, seven-day retention, fresh-directory restore, and matching-version rollback. Distinguish verified embedded execution from unperformed live-provider or physical-device checks. Keep model upgrades generation-based and HNSW disabled until a separately measured change.
- [ ] **Step 4 — Run final verification.** Run `cargo fmt --manifest-path backend/Cargo.toml -- --check`; strict locked Clippy; `cargo test --manifest-path backend/Cargo.toml --all-targets --all-features --locked`; `pnpm check`; `pnpm test:unit`; `pnpm build`; `pnpm test:build`; `pnpm verify:release`; and `pnpm test:e2e`. Run locked Cargo/pnpm native audits and triage findings by reachability. The native model smoke check is explicit and offline after setup; fake inference remains in routine unit cases. Verify both CI platforms or clearly record which hosted/platform checks remain unrun.
- [ ] **Step 5 — Commit and hand off.** `chore: verify embedded knowledge storage release and recovery`. Request the execution-method-prescribed whole-branch review, resolve its material findings, and report evidence. Integration/deployment follows existing user authorization; this plan does not authorize changing Tailscale, paid live research, or running migrations against the user's real library during tests.

## Coverage and self-review record

The tasks are sequential and share one set of types/contracts. Task 1 establishes embedded storage; Task 2 establishes all cross-task wire/identity types; Tasks 3–4 establish authoritative mutations; Tasks 5–6 add derived discovery; Tasks 7–8 preserve migration/recovery; Tasks 9–11 expose the complete behaviour; Task 12 verifies the actual release. No placeholder handler or production bypass is a task deliverable.

| Spec acceptance | Owning tasks |
| --- | --- |
| Cross-browser saved access; providers optional for reads | 3, 9, 10, 11, 12 |
| Shared identity versus context; reversible semantic connections | 2, 3, 6 |
| Atomic replay, refresh, revision, deletion and pins | 2, 3, 4, 9 |
| Local similarity, failure modes, no paid backfill | 5, 6, 8 |
| Bounded graph navigation; no research on saved opens | 6, 9, 11 |
| History, retained knowledge, explicit purge | 3, 4, 10, 11 |
| Preserved/idempotent browser migration and demo | 7, 10, 12 |
| Restart, backup restore and failed-upgrade recovery | 1, 8, 12 |
| Private access, input bounds and stale clients | 9, 10, 12 |
| macOS/Linux native release without Python/containers | 1, 5, 8, 12 |

Self-review completed on 2026-10-27: every acceptance requirement maps to tasks above, all five Review Focus conditions have owning tests, shared signatures were reconciled, and the plan is approximately 1.4 times the design's length. A check passing in the old HTTP spike cannot replace its planned embedded/application test. Most of this plan records interfaces and outcome assertions rather than prescribing function bodies; implementation belongs to the executor.

## References checked during planning

- [Embedded Rust setup and typed SDK conversion](https://surrealdb.com/docs/reference/rust/embedding).
- [SDK concurrency capacity](https://surrealdb.com/docs/reference/rust/concepts/concurrency).
- [SurrealDB crate metadata](https://crates.io/crates/surrealdb) and [fastembed crate metadata](https://crates.io/crates/fastembed), verified directly through the registry API on 2026-10-27.
- [Local Rust embedding integration](https://github.com/Anush008/fastembed-rs) and the [BGE model card](https://huggingface.co/BAAI/bge-small-en-v1.5).
