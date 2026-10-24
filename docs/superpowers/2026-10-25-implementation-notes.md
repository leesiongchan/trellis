# Implementation and review notes

The approved horizontal tree and reader are implemented on `feat/knowledge-tree`. The production Node server runs on loopback port 3000, with private Tailscale HTTPS forwarding on port 8443. The existing mockup preview on port 60090 is preserved. No public hosting or repository push was performed.

## Verification

- `pnpm check`: zero errors or warnings.
- `pnpm test:unit`: 32 passing tests across ten files.
- `pnpm test:e2e`: 13 passing browser scenarios, with mocked research responses.
- `pnpm build`: production build succeeded.
- Dependency audit: no known vulnerabilities in the installed dependency set.
- Private HTTPS runtime: access redirects with 303 and a Secure, HttpOnly, SameSite=Strict cookie; root responds with 200 and a content security policy. Anonymous research is rejected with 401, and an incorrect Origin is rejected with 403.
- The rebuilt, authenticated research endpoint responds with 503 / `SETUP_REQUIRED` because both provider keys are absent. The bounded live smoke exits without provider calls when credentials are missing.
- Desktop and 390px phone viewports were inspected through the private production endpoint. This does not establish physical-device behavior.

Live research remains unverified. Configure `OPENAI_API_KEY` and `TAVILY_API_KEY` in the ignored `.env`, restart the server and run `pnpm live-smoke`. Do not paste keys into chat or commit them.

## Final review

A fresh reviewer examined the whole implementation from `822d797` through `e5a92f3`. Both Important findings were fixed in one pass, with failing regressions observed before their fixes:

- An unreadable IndexedDB record previously prevented the whole exploration library from loading. Records are now validated individually; valid explorations remain available, the interface reports unreadable records, and the original unreadable data remains untouched. The valid-plus-corrupt-record unit test passed after reproducing the failure.
- Reloading an exploration deleted by another tab could leave stale work active under a new autosaver, recreating the deleted record. Reload now resolves the read before replacing the autosaver, cancels active research when deletion is confirmed, and returns home with a deletion notice. A failed read keeps current work open. Browser regressions for cross-tab deletion with a pending research result and a failed reload passed after reproducing the failures.

The complete unit and browser suites passed after this fix pass. No second review was dispatched.

## Rulings

1. Keep this brand-new project in the requested workspace on `feat/knowledge-tree` rather than creating a second checkout. No existing code or main branch needed isolation. Cost if wrong: less separation from concurrent edits, mitigated by status checks.
2. Use SvelteKit 3's Vite-plugin configuration instead of `svelte.config.js`. The installed framework release removed support for that file. Cost if wrong: configuration needs revisiting when changing framework majors.
3. Leave live provider/model availability, research quality and adversarial grounding explicitly unverified. The reviewer excluded live provider calls and credentials are absent; source checks are implemented and fixture tested. Cost if wrong: provider compatibility and the quality of real research remain unknown until the bounded live smoke and source review.
4. Accept the independently checked private HTTPS endpoint and desktop/phone browser viewports as runtime evidence. The reviewer did not independently assess Tailscale, physical phones or visual/accessibility behavior. Cost if wrong: device-specific or accessibility problems can remain despite the covered browser checks.
5. Accept the recorded dependency audit without a second audit, and make no large-tree performance claim. The reviewer did not independently establish dependency provenance or run load tests. Cost if wrong: unreported supply-chain issues or poor performance on very large explorations can remain.

## Deferred minor

Switching directly between two saved explorations in the library refocuses the destination's selected topic rather than restoring its exact saved pan and zoom. Saved content and selected topics remain available. A normal reload restores the saved viewport.
