# Server-only persistence

The user requested server-only storage and removal of demos. SurrealDB owns all persistent research content and exploration view state. The browser holds only the currently displayed state and unconfirmed requests in memory. Remove IndexedDB, `idb`, `fake-indexeddb`, demo UI/content, automatic legacy imports, and browser copy management. Leave existing browser databases untouched; browser-only data is no longer imported or displayed. Existing server imports and retained answers remain readable.

## Approach

Use the existing Rust API and embedded SurrealDB. Persist view state on the exploration record independently of content revisions, with last successful write winning across devices. Add authenticated GET/PUT exploration view endpoints with bounded validation and epoch checks. Discover pending server operations through a paginated authenticated operations endpoint and poll them after reconnect/reload. Keep uncertain requests in memory for same-tab retries with the same operation ID. Requests never admitted by the server cannot survive a reload; do not introduce another persistent client store.

An offline browser cache conflicts with server-only storage. Moving demo storage to the server was considered; the user chose to remove demos instead. Do not delete server-side import history or unrelated existing work.

## Behavior and validation

Always load the exploration list from the server on mount. A failed server connection reports an error and never restores content from browser storage. In-memory content may remain visible during a connection loss, with mutations disabled until reconnect. Show server view save errors and offer retry. Flush queued view changes before navigation. Continue existing auth, Origin/Host, protocol, request ID, body limits, idempotency, revision conflicts, and cancellation behavior. Validate selected/expanded appearance membership and finite bounded viewport values before storing views.

Prove server view persistence across reopen, cross-exploration rejection, stale epoch rejection, pending operation pagination, recovery without browser storage, and absent demo/browser-copy UI. Update frontend unit tests, browser tests, dependencies and operational docs. Verify check/build, frontend/build tests, Rust tests, and browser scenarios using disposable server data.
