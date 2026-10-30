# CI build reuse

The successful Linux run `41108947990` took 71m23s. Compilation accounted
for about 65m31s, including 4m03s installing cargo-audit. The user approved
starting with caching and comparing cold and warm runs.

## Approach

Use an immutable revision of Swatinem/rust-cache for `api -> target` and
installed Cargo binaries. Include Ubuntu 26.04 and cargo-audit 0.22.2 in
the key alongside the action's toolchain, environment, and dependency hashes.
Only successful main-branch runs publish caches; PRs may restore them.
Cargo still determines which artifacts are valid and all checks still run.

Keep the current two-job cap and release settings for this comparison.
Separate frontend verification from the native release step, add Cargo
timing reports to compilation commands, and retain the reports on failure
as well as success. Run CI once to populate the cache, then rerun the same
commit to measure reuse. Record observed durations rather than a target.

## Alternatives

Splitting independent checks into parallel jobs improves feedback but does
not eliminate repeated native compilation. Increasing compiler jobs needs
memory measurements first. Both remain follow-up options after this trial.
