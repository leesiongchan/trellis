# SurrealDB knowledge storage and topic connections

Status: approved by the user on 2026-10-27; ready for implementation planning. The user selected SurrealDB after the approved comparison spike and then approved this written design, including its proposed defaults. Implementation follows review of the implementation plan and selection of an execution method.

Builder check: a fresh software-engineering reviewer read the draft on 2026-10-27 and reported “nothing important left.” This checks design completeness, not implementation correctness.

## Intent and confirmed direction

Save questions and researched answers on the application server, and connect the same or similar questions, explanations, and branches across explorations. Preserve the original question, its context, citations, and research history when knowledge is reused.

The user requested persistent answer storage and connections between matching or related content, asked specifically about graph functionality and ArangoDB versus SurrealDB, approved a comparison spike, and selected SurrealDB for the design. No new hosting model, public community library, account system, or full-library graph screen was requested.

This is a project spanning persistence, research, browser migration, and navigation. It needs an implementation plan after approval of this document.

## Current application and evidence

The Svelte frontend stores entire exploration trees in IndexedDB through `src/lib/knowledge/persistence.ts`. Each node has exactly one parent, its own question, and optionally an explanation. The frontend generates node IDs, constructs research context from ancestors, and autosaves complete snapshots. Equivalent sibling questions receive limited text normalization; separate explorations cannot share topics. The Rust backend validates requests and coordinates research in memory, without storing completed answers.

The existing operating model is one private Rust service on the Mac, serving static assets and authenticated research, with optional access through Tailscale Serve. The previous approved designs are [the knowledge tree](2026-10-25-knowledge-tree-design.md) and [the Rust backend](2026-10-25-rust-backend-design.md). This design supersedes their exclusions of server storage and cross-exploration connections; their research-quality, private-access, and bounded-operation requirements continue to apply.

The throwaway comparison exercised ArangoDB 3.12.12 and SurrealDB 3.3.4 using 19 synthetic topics, 23 connections, two answer versions, and four queries with real local BGE embeddings. All 35 final functional checks passed. Both supported shared nodes, cycles, native shortest paths, uniqueness under concurrent writes, answer history, forced process restart, and native backup restoration. Exact retrieval rankings matched; restored vector indexes also worked. ArangoDB ranked weighted alternative paths in one native query; the SurrealDB implementation combined bounded path queries with application ranking.

The spike used HTTP containers, not embedded Rust. It establishes functional feasibility on a tiny fixture, not scaling, integration, or power-loss guarantees. Its files are local, ignored artifacts under `.superpowers/spikes/db-comparison-2026-10-27/`; durable evidence and reproducible application-level checks must be committed during implementation.

## Approaches considered

| Approach | Benefit | Tradeoff and decision |
| --- | --- | --- |
| PostgreSQL plus pgvector | One durable store for answers, relationships, and semantic retrieval | Graph traversal is implemented through relational queries. A viable alternative, but the user selected SurrealDB after discussing native graph support. |
| ArangoDB | Documents, explicit edges, vector retrieval, and native weighted alternative-path operations | Separate service, no officially maintained Rust driver listed, and applicable edition restrictions. Both engines passed the spike; advanced path ranking is not an established requirement. |
| SurrealDB | Documents, graph relations, vector search, official Rust integration, and an embedded deployment option | Chosen by the user. Embedded integration, storage durability configuration, upgrades, and release packaging need verification before shipment. |

SQLite and Neo4j were also researched. SQLite remains a simpler private-storage option; Neo4j offers a graph-centred model but requires additional modelling for nested answer documents. Neither was selected.

## Proposed defaults and scope

The following are assistant design choices, subject to approval of this document:

- One private library for this installation. All devices holding the existing access capability see that same library. There is no per-person isolation or public sharing.
- Embedded SurrealDB inside the existing Rust process, with persistent RocksDB storage. There is no exposed database HTTP/WebSocket listener and no additional runtime database service.
- Start with a maintained, pinned SurrealDB release compatible with the Rust toolchain; 3.3.4 is the spike baseline, not permission to use an unreviewed newer release. Use the same database version across the application, migration, and restore tooling.
- A shared topic graph underneath the existing exploration trees. The first interface exposes connections for the selected topic and links to their saved locations. A full-library graph canvas is deferred; exact layout of the connection controls belongs to implementation.
- Exact questions in the same saved context can reuse a topic automatically. Semantic equivalence and relatedness produce reversible connections between separate topics; they do not erase or globally merge existing records.
- Answers have immutable versions. Previously opened appearances remain pinned to the answer they showed until the user chooses an update.
- Embeddings run locally using the spike's English BGE model family as the initial candidate. There is no new hosted embedding service. Model/runtime integration and packaging must be verified; English similarity is the first supported retrieval language, while storing other-language content remains possible.
- IndexedDB remains a cache, migration source, and provider-free demo store. SurrealDB becomes the source of truth for research explorations after migration.

Goals are durable cross-device access through the existing server; useful connections across explorations; context-aware reuse; preserved provenance; safe import of existing browser data; and recoverable operation.

Non-goals are public hosting, accounts, collaboration, multi-server writes, offline editing and synchronization, automatic topic consolidation, recursively generated content, distributed graph analytics, and advanced weighted-path navigation. Similarity is not a factual-support guarantee, and a database migration must not weaken the existing evidence checks.

## Knowledge identity and answer history

A topic and its appearance in an exploration are separate records. A microSD branch and an SSD branch can reference one NAND topic while keeping different branch wording, reasons, positions, and selected answer versions. Appearance parentage remains a tree; topic connections may have multiple incoming edges and cycles.

| Record | Responsibility and invariants |
| --- | --- |
| Library | Installation identity and schema version. The namespace/database is fixed by server configuration, not chosen by browser requests. |
| Exploration | Original prompt, root appearance, mode, creation/update times, content revision, and deletion state. |
| Appearance | Exploration ID, parent appearance ID, topic ID, original question, bounded context snapshot, branch relationship/reason/evidence, sibling order, and pinned answer-version ID. An appearance belongs to exactly one exploration. |
| Topic | Stable opaque ID, canonical display question/title, current suitable answer-version reference, and creation/update times. Topic identity is not a normalized title. |
| Question variant | Original wording, normalized wording, context fingerprint, topic reference, and provenance. A unique exact lookup key covers library, research/demo mode, locale, normalized question, and context fingerprint. |
| Answer version | Topic reference, immutable validated content including nested source/evidence snapshots, original research input/context, generation metadata, timestamps, and revision. Refresh adds a record rather than overwriting an old explanation. |
| Connection | Typed SurrealDB relation between topics or between answer versions, with endpoint types, reason, origin, evidence or matching metadata, status, and timestamps. |
| Embedding | Reference to a question variant or answer passage, model/artifact version, dimensions, input hash, chunk location, vector, and indexing state. It is derived data and can be rebuilt. |
| Operation and research attempt | Stable intent ID, request hash, target, state, result reference, and timestamps. These support safe retries and interrupted-work recovery. |
| Migration manifest | Validated legacy content digest, import state, source provenance, record mappings, and acknowledged server revision. |

Keep sources and evidence scoped inside their answer version and cited branch reason. A URL appearing in several answers does not let a refresh rewrite older citations. A connection derived from two passages refers to their immutable version and paragraph IDs, so its explanation cannot silently drift.

Build the exact context fingerprint from a versioned, deterministic encoding of the validated question interpretation, nullable clarification, locale, and bounded ancestor input used for research. Store that input with the lookup. Do not infer context equality from a title, vector score, or model-generated sense label. Different exploration paths may therefore miss exact reuse and receive a semantic connection instead; this conservative behaviour is intentional.

Server-generated IDs and database uniqueness constraints enforce identity under concurrent requests. Use explicit typed records and indexes for exact keys, operation IDs, revisions, directed edges, and unordered symmetric pairs. Parameterize every query. No browser-supplied record/table names, SurrealQL, embeddings, matching verdicts, or timestamps are authoritative.

## Connections and semantic matching

The graph distinguishes navigation, equivalent meaning, and explanatory relationships:

- `branches_to`: a topic-to-topic link supported by at least one active exploration appearance pair. Keep individual branch reasons/evidence on the appearances; removing one occurrence must not remove another occurrence's support.
- `equivalent_question`: two differently worded topics judged to ask the same thing in compatible contexts. Preserve both identities and their original answers.
- `similar_question`: an automatically discovered similarity candidate, explicitly labelled as such rather than asserted equivalence.
- `overlapping_answer`: links two answer versions through the passage pairs that overlap. It does not imply that either complete answer replaces the other.
- `related_topic`: an explanatory relationship with a visible reason. A factual claim such as prerequisite, cause, or contradiction needs appropriate evidence and verification; a vector score alone creates only a similarity candidate.

Symmetric relations use one canonical unordered pair; directional relations preserve direction. Keep matching method, model/version, scores, reason, and accepted/dismissed state. Dismissal suppresses the same candidate for the same content/model versions. Fresh content can generate a new candidate without erasing the old decision. Do not create a topic's self-edge after exact reuse.

The matching flow is:

1. Normalize text without dropping meaningful distinctions such as negation, numbers, units, or product versions. Look up the scoped exact key. If it exists, attach a new appearance to that topic and an appropriate saved answer version.
2. For a new topic, embed its question plus bounded context locally. After successful research, also embed answer passages, including their headings and topic question. Similarity tasks run after the durable content commit.
3. Search questions against questions, answers against answer passages, and question-to-answer candidates as separate retrieval channels. Combine keyword and vector candidates; retain their provenance instead of treating raw scores from different channels as interchangeable.
4. Retrieve up to ten candidates per channel, collapse passage hits by target topic/version, remove self/demo/deleted/incompatible-model hits, and show at most five useful connections per selected topic by default. Search current topic answers and versions still pinned by appearances; unreferenced historical versions stay available in history without crowding default discovery. These are bounds, not quotas.
5. Optionally check at most three leading candidates in one bounded call through the existing model provider, as an explicitly attributable stage of the user's topic-research operation after its content commit. Persist a separate stage intent before this call. The caller can receive saved content immediately; restarting the app does not automatically resume an interrupted paid stage. The schema distinguishes equivalent, related, different, and uncertain and requires a reason. A failed or uncertain check leaves a labelled similarity candidate. No model-only verdict establishes a cited factual relationship.
6. Commit accepted equivalence or relatedness links with their provenance. This never rewrites two topics into one or substitutes a different answer without a user action. Provide a dismiss action for inappropriate connections.

The same mechanism connects suggested branch questions to saved topics. Unopened suggestions may receive local embeddings and labelled similarity candidates; they must not cause paid research or semantic-verification calls on their own. Similarity backfill and startup recovery also make no paid calls. Candidate comparison text is bounded, treated as untrusted data, and receives no tools or database privileges.

Use exact cosine retrieval initially. HNSW is a later, measured optimization behind the same interface, requiring comparison against the exact baseline with the application's filters. Pin model files, tokenizer, preprocessing, and dimensions together. Never compare vectors from different model versions. Changing a model builds a separate generation and switches only after backfill and evaluation.

Answer chunks follow paragraph boundaries with a tokenizer-derived maximum of 350 tokens plus bounded headings/context within the model's actual input limit. Process at most 64 chunks per answer initially; record partial coverage explicitly while retaining the full answer. Truncation can reduce discovery but cannot delete stored content. The concrete retrieval fixtures, false-link/false-equivalence acceptance rules, and thresholds must be established during planning; there is no universal cosine cutoff.

## Saving, research, concurrency, and navigation

The server owns content changes. Replace whole-exploration snapshot writes with commands for creating explorations/appearances, researching, refreshing, dismissing connections, and deleting explorations. The browser cannot overwrite a newer server answer by autosaving an old tree snapshot.

Every content command has a client-created operation ID reused across retries, a request hash, and the expected content revision where applicable. Atomically claim the operation before external effects. Same ID and payload returns the saved result or pending state; the same ID with different payload returns a conflict. Two different operations racing on an exact topic key converge through a unique constraint and reread. Database serialization conflicts receive a small bounded retry; provider calls are never repeated inside a transaction retry.

Research resolves the stored appearance and constructs its ancestor context on the server. Persist the intent before provider work. Preserve the existing search, generation, deterministic evidence matching, and independent grounding checks. After validation, one transaction saves the immutable answer version, updates the topic reference and selected appearance pin, creates the bounded branch suggestions and their appearance links, records operation completion, and advances the exploration revision. Return success only after that commit. Clarification and insufficient evidence remain distinct outcomes; incomplete answers never become active.

Different topic contexts have separate exact keys. Concurrent refreshes for one topic share one admitted attempt; a competing refresh cannot silently overwrite the first. Existing in-memory limits and cancellation remain useful, with a durable attempt record supplying recovery evidence. On restart, mark unfinished attempts interrupted, preserve their earlier content, and require an explicit retry. A provider timeout or crash after an unknown upstream outcome must not silently repeat the same paid intent. An explicit new retry is a new intent and may incur another provider request.

Refreshing one appearance creates a new answer version and pins that appearance to it. Other saved appearances retain their previous pins and can show that a newer version is available. New appearances may use the topic's current version only when their scoped exact context matches. Equivalent-question edges do not authorize automatic answer reuse across contexts.

Content revisions protect branch/delete/research mutations. View state is browser-local per exploration/device: selection, expanded appearances, viewport, and mobile pane do not create cross-device content conflicts. A browser with stale content reloads the current server snapshot after a revision conflict; it never silently replaces server state. Returning to a ready appearance uses its saved answer without a provider call.

Expose the selected topic's saved connections, their reasons/type, matching status, and existing exploration locations. Following one opens the destination appearance when available; a retained topic without a current appearance opens a read-only topic page with an explicit action to start an exploration there. Keep the original exploration available to return to. Large connection sets are paginated; do not render the entire library or recursively expand branches.

Demo explorations retain their current local behaviour and labels. They are excluded from the research graph, embeddings, equivalence, and research reuse. API credentials remain unnecessary for the demo and for reading already saved content with valid application access.

## Server and browser boundary

All stored-library reads and mutations require the existing signed access cookie. Mutations additionally require the existing exact Origin/Host checks. Apply bounds before database work. Keep the static shell and provider-free demo usable without a session; do not expose private library records through public SSR, metrics, asset paths, or a database port.

The following describe resource responsibilities; planning supplies exact typed request/response schemas and shared Rust/TypeScript contract fixtures before code:

| Resource | Required behaviour |
| --- | --- |
| Explorations | Authenticated paginated list; create; fetch a bounded appearance-tree snapshot; revision-guarded delete. |
| Appearances | Add a branch under a stored parent; reference a topic and selected answer version; select a newer compatible saved answer. |
| Research attempts | Start research/refresh for a stored appearance; return persisted completion, clarification, insufficient evidence, interrupted, or pending status. |
| Topics and connections | Read a topic/version; list bounded incoming/outgoing connections and saved locations; dismiss a candidate. |
| Operations | Read/replay the state of one authenticated command intent without repeating its effects. |
| Migration imports | Stage validated schema-v1 browser data in bounded JSON batches; finalize an import atomically; query its manifest. |

Use the existing camelCase wire style and `{ error: { code, message, retryable, requestId } }` envelope. Lists use stable cursors and a maximum page size of 100. Graph traversal is bounded by depth, node/edge count, and execution deadline; initial server limits are three hops, 100 nodes, and 200 edges per request. This is a private-library graph API, not a general SurrealQL endpoint.

Library requests use an additive protocol/version marker. The old `/api/research` request shape cannot safely establish authoritative server content from browser-provided ancestors; after cutover it returns a clear reload-required error rather than running unsaved legacy research. Existing v1 IndexedDB data remains importable. Document the page-reload requirement for open tabs during this release.

Retain operation deduplication records while their exploration exists. Deleting an exploration retains a non-content tombstone identifying its ID and completed deletion intent so delayed retries cannot recreate it. A browser resolves pending operations before issuing new writes; content mismatches return 409, unavailable storage returns a safe retryable error, and incomplete model matching does not turn successful content persistence into a failed save.

When the server is unavailable, cached completed research explorations remain readable in that browser. Disable research-content mutations and clearly label offline/cache state; there is no queued offline edit synchronization in this release. Provider unavailability does not prevent authenticated saved-library reads while the application server and database are available.

## Browser migration and deletion

After authentication, enumerate healthy schema-v1 research explorations in the current browser. Validate the complete legacy tree before staging it, preserve its node IDs as appearance identities in the migration map, preserve all answer/source/timestamp snapshots, and keep view state locally. Importing uses no provider requests. Unreadable explorations remain untouched with an actionable message.

Identify an exact legacy content snapshot by its legacy exploration ID and a stable digest excluding device view state and storage revision. Repeated imports, including identical copies from another device, return the same mapped result. A changed legacy snapshot with an already imported ID creates a separate preserved copy with clear provenance; it cannot overwrite a server exploration. Distinct legacy node answers become distinct immutable versions even where exact topic keys coincide.

Stage import batches under a manifest; nothing is visible in the active library or matching graph until complete structural validation and finalization. Atomically publish all mapped records and the manifest, then acknowledge the server revision. Proposed limits are 2 MiB per batch, 32 MiB and 2,000 nodes per exploration, with bounded concurrent imports. Oversized input is refused with its browser copy preserved and an export/recovery path; no truncating import is allowed. Planning must verify that these limits fit the current data model and define the bounded finalization transaction.

Keep the original IndexedDB records after successful import. Store the mapping/acknowledgement separately; ordinary cache updates must not replace the preserved v1 source. Detect later legacy-tab changes by digest and offer the preserved-copy import route. Never claim that another device's local-only data has migrated until that device actually connects and imports it.

Deleting an exploration removes its appearances and occurrence-supported navigation connections, and invalidates its device caches on the next authenticated revision check. Retain shared canonical topics and answer versions in the private library even when their last exploration is deleted. The confirmation explains this distinction. A separate library purge operation, with explicit confirmation, removes retained server content, related embeddings, and matches; it is included so deletion is possible without filesystem surgery. Advance a library epoch so pre-purge commands are rejected and clear connected-browser caches. Retain content-free import receipts/tombstones so automatic legacy imports do not recreate deleted content. IDs/tombstones needed to prevent replay contain no question/answer text. Offline devices and preserved legacy browser originals cannot be remotely erased: the purge confirmation explains that they need local clearing, and deliberate re-import requires an explicit restore action. Normal backup retention remains visible and documented.

## Embedded storage, operations, and recovery

Use one embedded datastore instance per process and share its client; never open the same directory independently for each request or test. Store it in an explicit absolute application-data path outside the release assets and repository. The application creates a restricted directory and fails clearly if it cannot obtain the datastore lock. A second application instance cannot open it for writes.

RocksDB is the proposed first persistent engine because the spike used it and the selected SurrealDB release documents support for it. Verify the chosen embedded API's synchronous durability configuration directly; the container's environment flag is not evidence of the Rust API setting. A saved acknowledgement must follow the durable commit. Resolve C++/libclang build requirements and packaged runtime libraries for macOS and Linux during planning. If this cannot preserve one application process, return with a concrete deployment tradeoff rather than silently introducing a sidecar.

Expose storage and embedding readiness separately. A failed database open or incompatible schema prevents library service readiness. A missing/corrupt embedding model disables discovery with a visible status while preserving saves and exact reuse. Use a bounded local embedding worker and a persisted derived-work cursor so startup can resume indexing without paid calls. Model files are installed explicitly with pinned hashes; application startup never downloads executable libraries or unpinned model files.

Version the database schema and migrations. Take and verify a backup before an upgrade; preserve the old release and its compatible backup for rollback. Do not point an older database version at a directory upgraded by a newer one. The supported rollback is restoring the pre-upgrade backup to a fresh data directory and running the matching old application release.

Provide operator commands in the Rust executable for consistent logical export and offline restore to a new empty directory. Daily backup while the service is available, a pre-upgrade backup, and retention of the last seven daily backups are proposed defaults; record the last completed/verified backup. Schedule failures surface in status/telemetry rather than pretending that a directory copy is a backup. Verify counts, graph references, pinned answers/citations, and representative vector retrieval after restoration. Preserve required model manifests; embeddings/indexes may also be rebuilt from source records.

During shutdown, stop admissions, let already admitted bounded operations commit within the existing grace period, stop derived workers, flush storage through the documented API, and close the database. Never hold a database transaction across provider or embedding inference calls.

## Privacy, limits, and observability

Private questions, answers, context snapshots, embeddings, and backups stay under the installation's access boundary. Embeddings are private derived content. Existing provider requests continue to send only bounded research context and selected candidate passages when semantic verification is explicitly part of a research action. No cross-installation matching, analytics, or third-party embedding service is introduced.

Validate browser data, legacy imports, provider verdicts, and database-version transitions at their boundaries. Escape text in Svelte; keep citations subject to current public-HTTPS validation. Model output cannot choose IDs, issue queries, or directly approve destructive changes. Imports accept only the application's validated JSON record format and never execute uploaded SurrealQL.

Bound content requests, import sizes, candidate counts, passage input, worker concurrency, transaction retries, and graph walks. Preserve current research limits unless a later approved change deliberately adjusts them. New storage/matching endpoints need their own bounded admission rather than consuming an unbounded work queue.

Extend existing tracing and metrics with storage latency/failures/conflicts, import progress, pending/failed embeddings, matching outcomes, graph-query bounds, interrupted attempts, and backup age/failures. Log operation/request IDs and safe codes; omit question text, answers, vectors, retrieved page text, cookies, paths containing secrets, and provider credentials. No content-bearing metric labels.

## Acceptance and implementation decisions

Planning must turn these outcomes into executable checks against the embedded Rust datastore and packaged application:

1. An answered exploration opened on one browser is readable from another authenticated browser through the same server; unavailable providers do not block saved reads.
2. Exact same-context questions share a topic across branches; identical wording with incompatible context remains separate. Semantic equivalence/relatedness/answer overlap create correctly labelled links without overwriting original answers.
3. Unique keys, operation replay, revision conflicts, concurrent refresh, deletion, and transaction rollback preserve all graph references and answer pins.
4. Similarity finds paraphrases and overlapping passages while retaining negation/context distinctions in its evaluation fixtures. Missing models or verifier failures preserve saves; backfill makes no paid calls.
5. Cyclic traversals terminate within limits; connections lead to useful saved locations. Reopening a saved topic does not initiate research.
6. Refresh preserves earlier versions and their citations. Removing a branch or exploration preserves shared knowledge; explicit purge removes content from the active library and its derived stores.
7. Schema-v1 imports are idempotent, preserve answers and IDs through mappings, handle changed copies without overwrite, and recover from interrupted staging without deleting browser originals. Demo and unreadable records stay isolated.
8. Forced process termination, restart, native backup restore, and failed upgrade recovery preserve acknowledged content under the selected durability configuration. Verify deployment-specific guarantees rather than inferring them from the spike.
9. Unauthenticated clients cannot read or modify the library; origin/host and payload/query bounds apply; stale legacy clients receive a useful reload instruction.
10. macOS and Linux release checks exercise embedded storage, local model assets/native runtime dependencies, startup from another directory, and clean shutdown. Production needs neither Python nor a database container.

The implementer may choose module/file organization, endpoint spelling, UI arrangement, record encoding, compatible crate versions, query/index details, and chunk batching within these contracts. They must verify embedded SurrealDB and local inference packaging early; changing the deployment model, hosted providers, sharing boundary, matching/merge semantics, or migration preservation rules requires a design revision.

## Sources

- [SurrealDB Rust embedding](https://surrealdb.com/docs/reference/rust/embedding) and [persistent storage](https://surrealdb.com/docs/running/file-backed).
- [SurrealDB transaction semantics](https://surrealdb.com/docs/reference/query-language/language-primitives/transactions) and [recursive path operations](https://surrealdb.com/docs/reference/query-language/language-primitives/idioms).
- [BGE model card](https://huggingface.co/BAAI/bge-small-en-v1.5) and [local Rust embedding library](https://github.com/Anush008/fastembed-rs). These support feasibility, not a verified packaging claim.
- [SurrealDB licensing](https://surrealdb.com/license) and [ArangoDB edition terms](https://arango.ai/downloads/).
