# Knowledge storage operations

## Storage foundation

The knowledge-library implementation uses embedded SurrealDB 3.3.4 with its
RocksDB backend. The application owns one datastore handle; there is no database
listener or separate database process. The foundation is being integrated through
the [implementation plan](../superpowers/plans/2026-10-27-surrealdb-knowledge-graph.md).
Later tasks connect this store to research and browser migration.

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
pinned public fixture. Release verification also runs the compiled native check
with restricted networking and a PATH without language runtimes or database CLIs.
