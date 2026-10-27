# Coding checkpoint — October 16, 2026

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

Historical checkpoint: development resumed on October 18; see [current UI workspace scope](ui-workspace.md) and [phase status](../phases.md). This checkpoint’s original evidence is preserved below.

User requested code completion through detailed roadmap P20, deferred deployment, then requested a pause at a reasonable checkpoint. This batch is a partial implementation. No phase is marked complete merely because these components exist. No deployment, live n8n write, real Guard call or credential export was performed.

## Resume here

1. Complete the remaining P06–P12 acceptance gates in [the runtime integration guide](p06-p12-runtime.md). The remote `018107d` commit already implements guarded shell/package/media execution, automatic deep review, scoped approvals, opt-in n8n reads and a dedicated TOTP service/UI. Preserve those paths; the checkpoint does not replace them. Remaining work includes full SAFE/safer-action integration coverage, sandbox/restart/package recovery and deployed write-gateway transactional/replay/reconciliation acceptance.
2. Finish main-chat file/image understanding, larger attachment ingestion and integrated artifact viewing. The remote includes bounded guarded image/video decoder workflows; full app acceptance remains open.
3. Complete thread tasks/artifacts/activity, runtime rich-card envelopes and supported actions, integrated pending security approval views, shared-memory editing/verification, automatic memory-review policy and cross-agent conversational capability integration.
4. Extend watch source monitoring through reviewed contracts, complete notification routing and confirm independent investigation/recommendation behavior end to end.
5. Verify the combined app in desktop/mobile browsers, then prepare complete service/VM acceptance. User deferred further deployment until coding is ready.

The first push exposed newer work on GitHub that the local checkout lacked. It was integrated without force-pushing. Earlier local-only test counts and gap descriptions are retained as historical evidence in the work log; combined-branch results supersede them.

## Added APIs

All operations below use the existing paired human operation endpoint or CLI `work`; human-only operations are explicitly denied by internal model dispatch even if a future capability grant is added accidentally.

| Operation | Inputs | Result |
| --- | --- | --- |
| `chat.page` | optional `thread_id`, `page: {before, limit}` | Newest-first messages with sequence, next cursor |
| `memory.page` | `filter: {query, source_type, minimum_confidence}`, `page` | Owned facts and exact review bindings |
| `memory.share` | `id`, `reviewed_hash` | Exact ordinary/current private fact copied into shared domain |
| `shared.page` | `page` | Explicitly shared facts with origin and provenance |
| `memory.unshare` | shared `id`, selected origin owner | Audited shared-copy revocation |
| `watch.settings` | none | Selected owner's settings; disabled by default |
| `watch.configure` | complete `settings` | Updates only that owner's read-only schedule |
| `notification.list` | `page` | Owned recommendations/failures |
| `notification.read` | `id` | Marks owned notification reviewed |
| `n8n.read` (remote integration retained) | `task_id`, named `workflow`, contract-bound `payload`, optional `request_id` | Policy/Guard/deep-review read and correlated safe result |

Page size defaults to 50; maximum 200. Cursors are positions and confer no ownership authority. Normal model context remains bounded to 50 chat messages. Private source/confidence filtering happens on the server before pagination. Relevant-memory retrieval searches across older records, excludes sensitive/stale/expired/low-confidence facts, and retains at most six facts per domain.

`POST /api/chat/:agent/stream` requires existing pairing and accepts the existing chat fields. SSE messages have `type: delta`, `complete`, or `error`. Providers with streaming emit real deltas; providers without streaming emit one final text chunk. Internal operation payloads are withheld. Disconnect/Stop drops pending inference; partial text is not committed. Completed internal operations remain durable, provider billing can be uncertain, and the conservative budget reservation remains charged. Refresh history/activity before resending. Reconnect currently retrieves persisted history rather than replaying an event stream.

## Configuration

`gateway.app_dir` optionally points to an absolute administrator-managed `app/dist`. It serves the built React frontend on the gateway origin, with no Node development server. Static path traversal, external symlink targets, unsupported MIME types and oversized files are rejected; API misses do not fall back to an HTML page. The static directory must never contain runtime state or secrets or be writable by an agent sandbox.

The administrator-owned `/etc/tinyclaw/security/execution.toml` and existing scoped credential broker remain the single execution/read/write configuration path. This merge discards the redundant local read-service adapter in favor of the remote's connected `n8n.read` service, capability grants, response projection, Guard auditing and deep-review path. See [runtime configuration](p06-p12-runtime.md). URLs and credentials remain unavailable as caller-controlled operation inputs.

Watch settings are managed under Activity. Quiet hours are explicitly UTC, not a guessed local timezone; equal start/end disables quiet hours. Daily limits use UTC days. Budgets are conservative reservations (microUSD), and normal model-router budgets remain in force too. A watch examines at most ten owned due/stale tasks and five failed-read references, creates one read-only investigation task, and spawns a worker with no tools or credentials. Concurrent hosts share atomic schedule claims and per-owner active-run checks. Recommendations appear only after investigation, or as a meaningful failure notice. Interruptions are retained rather than blindly replayed. Watches neither archive maintained threads nor dispatch external actions.

## State and verification

Schema version 7 adds watch settings/runs/notifications; explicit shared facts live in separate `data/shared.db`. Back up the complete stopped state tree before upgrading. Restore the matching backup to roll back; an older binary refuses a newer system schema.

Pre-integration local full Rust suites and the app production build/schema/API tests passed. Final combined-branch results are recorded in the work log. Formatting and whitespace checks pass. CLI/daemon fixtures use local mock services, not live model credentials. Detailed final check results are in `worklogs.md`. Pre-integration Rust 1.99 strict Clippy reported inherited diagnostics; the final combined-branch check is recorded separately. Browser/mobile, VM, rootless sandbox and deployed provider/workflow acceptance remain outstanding.
