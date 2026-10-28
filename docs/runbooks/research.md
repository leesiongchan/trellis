# Research failures

**Means:** a request failed at access, configuration, retrieval, generation, evidence checking, or its deadline. Earlier content remains readable; failed refreshes retain the previous explanation.

**First check:** find the reference shown in the interface in the server's JSON logs. `requestId` correlates events; Durable operation IDs connect a submitted command to its stored receipt. Logs omit questions, source text, URL keys and credentials.

| Code or condition                              | What to check                                                                                                                                                                |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ACCESS_REQUIRED` / 401                        | Open the private `/access?key=...` link again. The cookie expires after seven days; changing the access key invalidates it.                                                  |
| `ORIGIN_REJECTED` / 403                        | Browser address, `APP_ORIGIN` and the trusted reverse-proxy origin must agree, including scheme and port.                                                                    |
| `SETUP_REQUIRED` / 503                         | Both provider keys must exist in the server's ignored `.env` file. Restart after edits.                                                                                      |
| `SEARCH_FAILED` or `PROVIDER_LIMIT`            | Verify Tavily access, account limits and connectivity. Do not retry recursively.                                                                                             |
| `UPSTREAM_FAILURE` or `INVALID_EXPLANATION`    | Check provider access/model support and `generation_completed` timing. The API does not expose internal errors or echo raw upstream data.                                    |
| `GROUNDING_FAILED` / insufficient evidence     | Sources did not sufficiently support the generated claims, or the check did not finish. Try a narrower question or an explicit retry.                                        |
| `RESEARCH_LIMIT` / 429                         | At most two distinct jobs run at once, with ten new jobs per minute per process. Identical in-flight requests share a job.                                                   |
| `RESEARCH_TIMEOUT` / 504                       | Total deadline is 120 seconds; search has 20 seconds. A stalled provider may still have consumed a call; no automatic retry is performed.                                    |
| `BODY_TIMEOUT` / 408 or `BODY_TOO_LARGE` / 413 | The complete request body has ten seconds and a 65,536-byte cap. Check the client/proxy rather than increasing limits to hide stalled uploads.                               |
| `SERVER_DRAINING` / 503                        | Shutdown has begun; allow the supervisor's 130-second termination window. Retry explicitly after the replacement is ready.                                                   |
| Startup `INVALID_CONFIG`                       | Check the exact origin, loopback binding, access-key length, environment-file path and complete executable/assets/manifest release. Configuration errors omit secret values. |
| `OFFLINE`                                      | Check that the Mac, app process and Tailscale client are running. Saved content requires a server connection; only the currently open in-memory view remains visible.        |
| `WORK_INTERRUPTED`                             | The server restarted during paid work. Previously completed content is preserved; retry is explicit.                                                                         |
| `RELOAD_REQUIRED` / 426                        | An older frontend called the legacy endpoint. Reload the paired protocol-2 frontend before any new research.                                                                 |
| `REVISION_CONFLICT` / 409                      | Reload the saved version. A restored/purged library has a fresh epoch; stale commands cannot replay.                                                                         |
| Discovery unavailable                          | Check pinned model assets and hashes. Saved answers still open; startup never downloads models or starts paid backfill.                                                      |
| Saving failure                                 | Keep the tab open. Restore the server connection and check server storage, then use Retry saving.                                                                            |
| Changed in another tab                         | Reload the saved version explicitly. The app refuses stale overwrites.                                                                                                       |

**Escalate:** reproduce once with the reference ID, stage and elapsed time. Keep secrets and page contents out of reports. Run the fixture-backed checks before changing the providers. Use the two-request live smoke only when provider configuration is ready.

Runtime scope is one Rust process on loopback with private Tailscale forwarding. The embedded library persists operation receipts and immutable answers across restarts. Limits and storage ownership are single-process; this is a private shared library, not a distributed or multiuser service. Public hosting requires a separate design for access control and shared limits. See [Rust service operation](rust-service.md) for configuration, release checks, metrics, graceful shutdown and rollback.
