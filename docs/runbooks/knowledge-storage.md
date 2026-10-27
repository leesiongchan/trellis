# Knowledge storage operations

## Storage foundation

The knowledge-library implementation uses embedded SurrealDB 3.3.4 with its
RocksDB backend. The application owns one datastore handle; there is no database
listener or separate database process. Research persistence, immutable answer versions, reversible connections and browser
migration share this handle. Deploy the protocol-2 frontend and backend together.

`APP_DATA_DIR` is an explicit absolute directory dedicated to this installation.
Keep it outside the repository, release assets, model directory, and backups.
`APP_BACKUP_DIR` defaults to a sibling `backups` directory. `APP_MODEL_DIR`
defaults to `models` beside the executable. Existing path ancestors are resolved
before overlap checks; dot traversal and symlink escapes into reserved roots are
rejected.

The data directory has mode 0700 and an ownership marker. The application refuses
to adopt an unrelated nonempty directory. A nonblocking process lock protects the
entire data directory, in addition to RocksDB's own lock. Do not delete an apparent
lock file to force a second writer; stop the process that owns the datastore.

The embedded connection uses the documented `sync("every")` option, which requests
disk synchronization on each committed transaction. No process environment is
mutated to configure durability. The application closes the SDK handle after
outstanding local queries finish; SDK teardown is asynchronous, so reopening may
briefly wait for the engine lock. An already acknowledged synchronous commit does
not depend on shutdown to flush its data.

## Native build prerequisites

Build with the repository's pinned Rust 1.98.0 toolchain. RocksDB compiles native
C++ and uses bindgen, so a C++ compiler and libclang are build requirements.
On the development Mac, Apple clang 21 and the Command Line Tools libclang at
`/Library/Developer/CommandLineTools/usr/lib/libclang.dylib` were available.
Use `LIBCLANG_PATH` explicitly if bindgen cannot locate that library.
On Linux, provide the corresponding compiler, clang/libclang, and normal native
build tools through the environment or Nix. Verify platform-specific runtime
linkage rather than copying build-machine libraries blindly.

The storage foundation's checks are:

```sh
cargo test --manifest-path backend/Cargo.toml --test storage --lib storage --locked
cargo test --manifest-path backend/Cargo.toml --locked
cargo clippy --manifest-path backend/Cargo.toml --all-targets --all-features --locked -- -D warnings
cargo build --manifest-path backend/Cargo.toml --release --locked
```

The integration tests cover validated paths, ownership and exclusive open; the
private module test covers nested answer/source round trips without exposing a
general query endpoint. All tests use owned temporary directories.

## Local discovery model and native runtime

Install the pinned public model explicitly before packaging:

```sh
node scripts/fetch-embedding-model.ts .artifacts/embedding-model
pnpm build
```

The setup script verifies each SHA-256 from
`tests/fixtures/library/model-manifest.json` and uses the immutable model revision.
`pnpm build` includes a verified `.artifacts/embedding-model` when present; an
explicit fifth argument to `scripts/package-release.ts` selects another model
source directory. Model assets and their manifest live in `release/models/`,
outside `public/`. Missing or corrupt assets disable discovery while the saved
library remains available. Startup performs no model downloads.

Fastembed 7.5.0 uses pinned ort 2.0.0-rc.15 / ONNX Runtime 1.31.0 with static
linking. `native-runtime-manifest.json` records the native distribution archives
and hashes. Current distribution targets are Apple Silicon macOS and x86-64 or
AArch64 Linux. Inspect the actual executable's platform linkage at release time;
no additional runtime library files are needed by the verified static build.
The x86-64 model loader checks AVX2 support before initializing inference.

The small [vendored portability patch](../../backend/vendor/fastembed/BRANCH-PATCH.md)
adds an optional safe CPU device policy. Branch selects CPU with two inference
threads. This prevents ort's automatic CoreML model-cache compilation on macOS.
The patch keeps upstream defaults unchanged for other callers and records the
original crate archive digest for review and eventual removal.

Query and document inputs use the public spike's raw-text preprocessing, CLS
pooling, and L2 normalization. The tokenizer and preprocessing version participate
in model identity. Paragraph chunks contain at most 350 passage tokens, remain
within the model's 512-token input, and retain paragraph locations. At most 64
chunks per answer are indexed; the full answer is retained and partial coverage
is recorded. One blocking inference batch contains at most 32 inputs.

Derived indexing records a source/model/input-hash state and a durable cursor.
Cancellation leaves unfinished sources available for restart, and model readiness
requires the backfill to finish. Discovery reads use streaming exact cosine in
bounded pages with a total timeout. Default passage candidates include current
answers and versions still pinned by appearances.

Verify with:

```sh
cargo test --manifest-path backend/Cargo.toml --locked --test embeddings --test embedding_index
BRANCH_EMBEDDING_MODEL_DIR="$(pwd)/.artifacts/embedding-model" cargo test --manifest-path backend/Cargo.toml --locked --test embeddings native_embedding_matches_pinned_public_fixture -- --ignored
node --test tests/build/package-release.test.ts
```

The native acceptance check compares query and document vectors against the
pinned public fixture. The explicit native parity gate can run its compiled test executable with restricted networking and an empty PATH. Packaged-release verification separately loads the native model with an empty child PATH.

## Startup, recovery, and readiness

Set `APP_DATA_DIR` explicitly before starting the service. Data and backups must be
outside every Git checkout, public asset root and model directory. The store
normalizes roots even for typed internal callers, so native engine paths remain
stable across aliases such as macOS `/var` and `/private/var`.

The ownership marker and `.branch-storage-version.json` protect the directory.
An incompatible engine marker is rejected before opening RocksDB; an incompatible
database schema is rejected before changing schema definitions or serving library
requests. Never edit markers to bypass compatibility checks.

Startup marks unfinished research operations and optional paid connection checks
as interrupted. It releases stale guards and resumes only local derived work.
Saved answers and appearance pins remain intact. An interrupted paid stage needs
an explicit retry; startup and indexing never send provider requests. Missing or
corrupt model files disable discovery while storage and saved library reads remain
ready. Startup verifies assets without downloading models.

`/health/ready` reports storage/library readiness separately from discovery and
provider configuration. Authenticated metrics include `branch_storage_ready`,
`branch_library_ready`, `branch_discovery_ready`, `branch_backup_age_seconds`
(`-1` before the first success), `branch_backup_overdue`, and
`branch_backup_failed`. Metric labels and logs contain no private content, vectors,
filesystem paths, cookies or provider credentials.

Shutdown drains the existing research budget, cancels background scheduling,
waits for the one local inference batch, and closes storage. Existing 125-second
grace and 130-second hard deadline remain in force; runtime teardown also has a
bounded wait for blocking tasks. Synchronous commits do not depend on shutdown.

## Backups while the service runs

The runtime uses its existing handle for native SDK exports. The first eligible
start creates a snapshot; a running service then checks once per minute and
creates the next snapshot when the last completed backup is at least 24 hours
old. Busy paid work defers export. A maintenance lease pauses mutations and
local derived writes, while library reads remain available. No transaction spans
inference or provider HTTP.

Each completed private backup directory contains `database.surql`,
`manifest.json`, and an ownership marker. Files have mode 0600 and directories
0700. The manifest records engine/schema/model versions, source library identity
and epoch, source application checksum, record counts, export byte length and
SHA-256. Completion uses fsync and an atomic rename that refuses to replace an
existing destination. Incomplete owned export directories are cleaned up on
failure or cancellation. Never copy an open RocksDB directory as a backup.

Keep seven completed daily snapshots for this library. Separate `pre-upgrade-*`
snapshots remain outside daily retention. Before a compatible application release
changes, startup takes a pre-upgrade export labelled with the previous release's
checksum. Retain that old release bundle alongside its backup. Unsupported engine
or schema transitions stop startup and require a supported migration; do not point
another engine version at the live data directory.

Backups contain private questions, answers and citation snapshots. They are not
public assets. Purging the live library does not erase retained backups, exported
files, or offline browser originals. Clear those separately when required. Seven
completed snapshots can span more than seven calendar days if the app was stopped
or backups were deferred; monitor age and failure metrics.

## Standalone backup and restore

The standalone backup command requires the service stopped. It takes the same
exclusive directory lock and refuses a second writer. It needs storage settings,
without requiring an origin, public assets or provider credentials:

```sh
APP_ENV_FILE='' APP_DATA_DIR=/private/branch-data release/branch-backend storage backup --output /private/branch-backups/manual-snapshot
```

Use an absolute output outside data, public assets, models and repositories. The
destination must not exist. The backup command prints the completed manifest,
which contains metadata rather than question or answer text.

Restore into a **nonexistent** absolute destination, which the command creates
empty. Verify the selected backup and retain its matching application release:

```sh
APP_ENV_FILE='' release/branch-backend storage restore --input /private/branch-backups/manual-snapshot --data-dir /private/branch-restored
```

Restore validates manifest format, engine/schema compatibility, export hash and
length, source library identity/epoch and record counts before declaring success.
A durable in-progress marker prevents an incomplete restore from being served.
If restore fails, preserve the original backup and use another fresh destination;
inspect and discard the failed owned target separately. Never delete the marker
to treat an incomplete restore as a valid library.

The restored library gets a fresh epoch. Old browser caches, paid operation
intents and automatic legacy imports cannot replay against it. Previously saved
content and pinned versions retain their IDs. Deliberate re-import of a preserved
browser original requires the explicit restore action.

Rollback means restoring the pre-upgrade backup into a fresh directory and
running the matching old application release with `APP_DATA_DIR` pointed there.
Do not reuse a data directory that a newer engine has upgraded. Validate counts,
representative nested answers/citations, appearance pins and retrieval before
switching traffic. The native automated checks cover this fresh-directory path.

## Native maintenance checks

The 32 MiB import gate runs separately from compiler jobs to measure application
work rather than build contention:

```sh
cargo test --manifest-path backend/Cargo.toml --locked --test migration thirty_two_mib_native_finalization_is_atomic_and_bounded -- --ignored --nocapture
cargo test --manifest-path backend/Cargo.toml --locked --lib --test backups --test storage_lifecycle --test lifecycle
```

The verified 32 MiB public fixture published 1,000 immutable answers atomically in
7.57 seconds and read back all appearances/answers. The standalone debug test
peaked at approximately 771 MiB RSS on the development Mac. Leave memory headroom
for imports and local inference; this is a measured fixture, not a production
capacity promise. Ordinary requests still use the documented graph/list limits.

## Browser upgrade and saved navigation

Keep the existing scheme, hostname, port and access key. Authenticated protocol-2
clients import v1 research originals in bounded JSON batches, atomically publishing
only a complete validated tree. Originals and demos are untouched. Changed original
content becomes a separate labelled copy; invalid/over-32-MiB or over-2,000-node
originals stay on the device and can be exported. Import receipts plus the library
epoch prevent deleted or purged content from returning automatically. Restoring a
preserved original is an explicit action, creating another copy.

The shared cookie grants access to the entire library; there is no per-user
isolation. A second device with private access can list/resume saved explorations.
Offline copies are acknowledged, read-only snapshots. Pending intents recover by
operation ID after reconnection, without queuing new offline mutations. A stale
revision requires explicit reload; purge/restore invalidates old cache scopes.

Question wording and bounded resolved context determine exact identity. Semantic
matches never substitute an answer. Accepted edges retain original model evidence
through model upgrades; dismissed content pairs remain dismissed across model
generations. New answer versions can produce new candidates. Model/index upgrades
are generation-based; old vectors are never mixed with the new generation. Exact
cosine search remains enabled; HNSW requires a separately measured change.

Saved links open existing appearances or retained knowledge without paid work.
Answer history is immutable and each appearance retains its chosen pin. Refreshing
one appearance never advances the others. Deleting an exploration retains topics
and answers; confirmed purge removes live content and derived records. Purge does
not clear backups, exported files, or browser originals: use the separate controls
and retention procedure for each.

All lists are capped at 100; graph walks at 3 hops, 100 nodes, 200 edges and two
seconds. Research uses the original 120-second budget, two concurrent jobs,
10 new jobs per minute and at most 32 waiters. A commit may optionally verify up to
three candidate pairs in one existing-provider call; local indexing performs no
paid backfill. Import staging allows two active manifests and finalization admits
at most two preparations, retaining admission even when blocking parsing outlives
its cancelled caller.
