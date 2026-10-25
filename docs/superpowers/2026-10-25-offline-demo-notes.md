# Offline demo handoff

The home screen now offers **Try the microSD demo**. It creates a fresh saved copy of eight prewritten topics, with a path from microSD cards through NAND flash and memory cells to quantum tunneling. Other branches cover the controller, error correction, pages/blocks and wear leveling. Primary-source links are bundled with the explanations.

The workspace and library identify demo content. Expanding, reading, refreshing programmatically, clarifying and adding questions cannot invoke research providers in demo mode. New questions stay as unanswered local branches. Switching explicitly to a research exploration retains the original research behavior. Demo interactions require no provider credentials; loading the app still requires its server, and external source links require connectivity.

## Verification

- 35 unit/integration tests passed; legacy saves without mode default to research.
- 17 browser scenarios passed, including demo persistence, zero demo research requests, explicit mocked transition back to research, independent question drafts and phone navigation.
- Type checking reported zero errors or warnings; production build and changed-file formatting checks passed.
- Desktop and 390px phone views were inspected. The phone library button was clipped before its regression fix and is now fully within the viewport.
- The button waits for browser initialization; an immediate click cannot be lost before its handler attaches.
- The private production app was rebuilt and restarted on its existing loopback port and Tailscale route. Demo branches were inspected through that endpoint. No live research verification or provider calls were performed by this work.

## Review

One fresh read-only reviewer examined `3ff1288..c7604c9`. The review found no Critical issues and one draft-isolation issue labelled Minor. It was regraded Important because an unsubmitted question could attach to the wrong exploration after switching demo copies. A browser regression reproduced the leak; the reader is now keyed by both exploration and selected-node IDs. The regression and full suite passed afterward. No second review was dispatched. No review findings remain deferred.

## Rulings

1. Continue in the accepted feature workspace, extending the same unmerged application with no concurrent edits. Cost if wrong: less isolation from concurrent workspace changes.
2. Treat approval to build the concrete demo proposal as authorization to proceed with its short design and plan. This preserves user authorization without repeated confirmations. Cost if wrong: implementation choices may need revision after use.
3. Preserve custom questions as unanswered local branches because the user excluded real generation. Cost if wrong: arbitrary offline answers require another feature.
4. Leave live provider correctness and provider-backed source retrieval unverified, as the user explicitly excluded real generation and live verification. Local guards and a mocked research transition are covered. Cost if wrong: the actual provider workflow is not established by this handoff.
5. Keep server access necessary for app reload because offline installation/service workers are excluded and documented. Cost if wrong: the demo cannot reload while disconnected from its app server.
