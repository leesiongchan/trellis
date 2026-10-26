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
