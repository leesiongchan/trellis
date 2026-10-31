# CI build reuse

The successful Linux run `41108947990` took 71m23s. Compilation accounted
for about 65m31s, including 4m03s installing cargo-audit. The user approved
starting with caching and comparing cold and warm runs.

## Approach

Use an immutable revision of Swatinem/rust-cache for `api -> target` and
installed Cargo binaries. Include Ubuntu 26.04 and cargo-audit 0.22.2 in
the key alongside the action's toolchain, environment, and dependency hashes.
Only main-branch runs whose packaged native build passed publish caches;
PRs may restore them. Save completed dependency artifacts even if browser
acceptance later fails. Early audit/compilation failures do not publish
partial immutable caches. The action reads `save-if` during its post step;
the runner reevaluates inputs then, including a completion flag set only
after the packaged native build and its runtime checks pass.
Cargo validates artifacts on reuse and failed checks still fail the run.
Cargo still determines which artifacts are valid and all checks still run.

Keep the current two-job cap and release settings for this comparison.
Separate frontend verification from the native release step, add Cargo
timing reports to compilation commands, and retain the reports on failure
as well as success. Run CI once to populate the cache, then rerun the same
commit to measure reuse. Record observed durations rather than a target.

The first trial passed all native checks but failed a browser interaction
after 73m08s. Success-only cache saving discarded the completed dependency
work, so the follow-up explicitly enables cache saving on verification
failure. Browser failure evidence remains retained and blocking.
The two-tab browser setup now waits for the initial save acknowledgement
before opening the second tab, avoiding startup recovery of pending initial
research. The scenario passed 10 repeated runs and the full 32-test browser
suite passed locally; application behavior and test timeouts are unchanged.

A second trial stopped before compilation on GHSA-g3hv-75wg-m249. Update
only minimatch 10 consumers to the published fixed 11.0.3: the installed
ESLint, config-array, and typescript-estree consumers use matching APIs
preserved in 11. No 10.x backport is published. Verify lint selection,
types, frontend build, and tests; retain the existing deepmerge exception.
Remove that trial's partial cache before testing the complete-build policy.

References: [minimatch advisory](https://github.com/advisories/GHSA-g3hv-75wg-m249),
[runner post/input evaluation](https://raw.githubusercontent.com/actions/runner/main/src/Runner.Worker/ActionRunner.cs).

## Alternatives

Splitting independent checks into parallel jobs improves feedback but does
not eliminate repeated native compilation. Increasing compiler jobs needs
memory measurements first. Both remain follow-up options after this trial.
