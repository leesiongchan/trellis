# Fresh-install audit

Audited the published README at `fed264d` on macOS 27.2 / Apple Silicon,
using two disposable clones outside the working repository. The application’s
existing `.env`, data, and release files were not used.

## Finding and fix

Running `pnpm check` immediately after a locked dependency installation failed:

```text
config_load_error
[TSCONFIG_ERROR] Failed to load tsconfig '$app/tsconfig': Tsconfig not found
```

The app’s check script invoked Panda before SvelteKit generated the TypeScript
configuration Panda needs to load its config. The production frontend build
already initialized these correctly, which hid the problem in existing checkouts.

Changed `app/package.json` to run `svelte-kit sync` before `pnpm styles:codegen`.
Verified the fix in the second clone with both `app/.svelte-kit/` and
`app/styled-system/` absent. The complete `pnpm check` command then passed.
Existing CI runs this command before building, so it exercises the corrected path.
No generated output or dependencies changed.

## Verification

| Check | Result |
| --- | --- |
| README clone and pinned mise tools | Passed; tool versions already installed |
| Locked dependency installation with scripts disabled | Passed in both clones |
| Explicit public model download | All five fresh downloads matched pinned SHA-256 hashes |
| Complete `pnpm build` | Passed from an empty Cargo target directory; native release compilation took 5m 20s |
| Clean-checkout `pnpm check` | Failed before the fix; passed after it |
| Frontend unit tests | 45 passed |
| Build and packaged-runtime tests | 5 passed, including no-Node runtime, persistence, backup/restore, model loading, and zero paid provider stages |
| README `.env` setup and `pnpm start` | Passed using an OpenSSL-generated key and temporary private storage |
| Readiness | Storage, library, and local discovery ready; research intentionally unconfigured |
| Private access | Unauthenticated library request rejected; generated access link accepted |
| First-use browser flow | Welcome screen works; missing provider keys produce an actionable message and retain the question |
| Restart | Retained question reopened after a graceful stop and restart |
| Phone layout | Home, Map, and Read views checked at 390px; no document overflow |
| Automated browser accessibility audit | Zero reported violations on the setup-error screen |
| Provider usage and logs | Zero provider stages; generated key absent from logs |

Both audit servers shut down gracefully. No real provider credentials or
application data were used.

## Limits and existing diagnostics

- This verifies a fresh source checkout with cached mise tools, pnpm packages,
  and Cargo registry inputs, not a machine with empty global caches.
- Linux installation and live OpenAI/Tavily research were not exercised.
  No paid calls were made, and no live cited answer was claimed.
- `pnpm check` retains four existing `no-floating-promises` warnings in
  `tests/site/trail.test.mjs`; it exits successfully with zero errors.
- The frontend build retains the existing generated pure-annotation and
  colour-environment warnings. They did not prevent a valid release.
- The automated accessibility audit is a partial check, not a complete
  accessibility certification.
