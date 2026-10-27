# UI workspace scope

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

Last reviewed: 2026-10-20 (America/Edmonton). Phase 5 app completion is next; this guide does not declare P16–P20 or live deployment complete. Latest local build/14 unit/API checks/67 desktop/phone browser checks pass with one intentional skip. Earlier counts/tool versions below are dated evidence. Phase 4 provides human-only shared-memory save/verify/stale backend contracts and source-linked watch notifications; new editing/evidence/conversational app controls remain Phase 5 work.

## Entry points

| Entry | Purpose | Data and authorization |
| --- | --- | --- |
| `/` | Canonical current workspace | Live chat, tasks, calendar, thread files/results, Activity/watch controls, Approvals, Memory and settings; same-origin loopback or paired access |
| `/prototype.html` | Compatibility alias | The same application and bundle as `/`; no separate management shell |
| Either URL with `?mode=sample` | Explicit design preview | Fictional browser data only, selected deliberately |

Run the Vite app using [app/README.md](../app/README.md). Both entry points build into the same `dist` directory. Development proxies `/api` only to loopback `127.0.0.1:8080`; production uses gateway same-origin static serving or an administrator-configured authenticated reverse proxy/tunnel.

## Global settings and agent setup

These are separate scopes with one entry each:

- **Global app settings:** the explicitly labeled App settings footer with gear icon, or Command/Control + comma. Appearance, optional workspace notices, calendar preferences, gateway connection, shared Claude status and Fast/Normal/Deep model profiles (sample mode retains browser drafts). No duplicate workspace-navigation Settings item.
- **Agent settings:** control beside Janet or Harley. One page for that owner’s Identity & memory, Skills, Credentials, and Access & boundaries. Identity/personality is edited in `soul.md`, selected memory notes in `memory.md`; no second personality form. Skills and credential previews do not have standalone workspace entries. Files contains conversation/project filenames.

Shared API/profile settings do not grant tool capabilities or cross-agent access. Explicit sample source-file previews do not change runtime identity, policy, or semantic memory. Live settings save through the dedicated backend and apply owner guidance to subsequent turns. Development uses host environment credentials. Production web credential enrollment and Vault remain disabled; no Claude API key/password/token is collected by settings forms. Access descriptions are required platform boundaries, not a live permission inspection.

## Tasks, calendar and threads

Sample tasks support create/edit, description/checklist, priority ordering, optional deadline, thread association, completion/reopening, and active/completed filters. Calendar deadlines derive from task state. Calendar preferences cover week start, 12/24-hour format and completed deadlines; sample events can be created/edited. Sample today remains October 16, 2026; event times are labeled America/Edmonton. These events do not synchronize or send invitations.

Each Focus Thread has Conversation and Thread workspace views. Linked tasks, attachment filenames, User-recorded decisions, running sample workers and completed fictional handoffs stay in an agent/thread key. Attachments retain names only; no contents are read or uploaded in sample mode. Closing a sample worker records its fictional result in the thread and Activity.

Connected mode now uses the workspace sidebar and Janet/Harley identities. Create Focus Threads through `thread.create`; the server ID, rather than the title, binds conversations, tasks and file metadata. A thread shows its persisted summary, linked tasks, upload/download controls, durable human decisions and structured worker results with archive downloads together. Earlier conversation pages use the server's `chat.page` cursor. Fast/Normal/Deep in the composer selects an actual runtime profile for that request; browser-local shared model drafts still do not activate server configuration.

Activity/file metadata load separately from chat/tasks, so an unavailable secondary endpoint reports a partial failure without blocking conversation. Navigation clears the previous owner's view and cancels pending reads. On desktop Enter sends and Shift+Enter inserts a newline; touch Enter inserts a newline and Send submits. Unconfirmed thread creation blocks resubmission within the form and requires checking the refreshed list before retrying.

Connected tasks use `task.create`, `task.edit` and `task.status`. `task.edit` validates owner, title/description, thread ownership, priority and RFC3339 deadlines, rejects terminal tasks, and commits fields plus task-history event atomically. Live completed tasks cannot reopen; the runtime requires a fresh retry task. Date-only deadlines become the end of that day in America/Edmonton with the correct daylight-saving offset. Unchanged deadlines retain their original instant.

Creation and detail editing are separate requests. A known created ID is retained if detail saving fails. Unconfirmed creation disables blind resubmission until the dialog closes and refreshed tasks are inspected. No automatic mutation retry is performed.

## Local development access

User requested no gateway token for current local use. Start `cargo run --locked -- gateway --host 127.0.0.1 --port 8080 --local-ui` (or use the same flag with `daemon`). The mode is off by default and can also be configured with `[gateway] local_ui = true`. It refuses non-loopback binds, configured tunnels and trusted proxy headers. Token-free requests must have a loopback Host, a loopback browser Origin when present, acceptable fetch metadata and no forwarding headers. Non-browser local clients may omit Origin. Never expose this development mode through a reverse proxy.

Both browser entry points omit the token form and bearer headers on localhost. Root-app reload reconnects locally; Disconnect clears view/cache and offers token-free Connect. On remote hostnames, pairing forms remain. Disabling legacy webhook pairing alone still does not open the assistant API. TOTP authorization, Guard, policy, secret handling and peer boundaries remain enforced independently.

## Storage and credentials

| Data | Lifetime / scope |
| --- | --- |
| Sample chat, tasks, events, decisions and worker results | Tab memory; reset on reload |
| Sample skills | Explicit local-storage saves under `raaviclaw.prototype.skills.v1.<agent>` |
| Sample identity/memory source files | Explicit local-storage saves under `raaviclaw.prototype.core-files.v1.<agent>` |
| Sample model/profile and budget drafts | Explicit local-storage saves under `raaviclaw.prototype.model-draft.v1`; closed nonsecret schema only |
| Live global preferences/profiles/budgets and owner soul/memory notes/skill text | Versioned schema-v9 backend records; durable across browser/runtime restart |
| Paired gateway token (remote only) | Dedicated password form; React memory only; cleared on disconnect/reload |
| Raw provider/integration credentials | Never accepted by the preview; requires dedicated secure backend enrollment |

Local storage is a UI convenience, not server-side isolation or encryption. Invalid saved files/model drafts are preserved and editing/saving disabled where necessary; storage failures leave edits visible. Sample model-draft saves do not activate live configuration; live global settings save active profile/budget configuration to the backend. Models must be selected from the allowlist; output limits are integers 1–32,000, budgets US$0–10,000 with at most two decimal places. Zero blocks usage rather than granting unlimited spend. Unknown fields, including secret fields, are rejected.

Disconnect and agent switching abort active reads and discard previous browser view state. Disconnect shows the connection screen; sample mode is available only through an explicit choice. Already-dispatched server work may finish. Unconfirmed chat responses instruct the user to refresh history before resending. Tokens/codes are not stored in browser storage, URLs, artifacts or model messages. Approvals open inside the current workspace and retain the dedicated authenticated n8n/TOTP path.

## Remaining integration

The connected view shows paginated chat history and owned task/thread lists (currently unpaginated). Production web credential enrollment, external-calendar synchronization, main-chat image understanding, browser push notifications and provider billing verification remain unfinished. Global settings open from the live User footer and per-agent settings from that agent’s gear. Live configuration is persisted independently of explicit sample browser drafts. Existing app/backend functionality should not be inferred from sample cards. Shell, external writes, peer access and persistent watches are not enabled by UI drafts.

## Verification and delivery

Frontend build and 14 unit/API tests pass. Full local Rust all-target tests pass (2,434 executions, five host-dependent sandbox tests ignored); tiny and standard feature suites also pass. Local Rust is Homebrew 1.99, so its new Clippy lints are not evidence for the required Rust 1.92 CI gate. Desktop/phone browser validation passes (39 checks; one intentional desktop skip), with the six live-workspace checks rerun after the final partial-outage changes, with screenshots written to portable per-test output directories, and the release build passes. Final-head GitHub CI results are recorded in [merge readiness](merge-readiness.md) and [worklogs](../worklogs.md).

GitHub was fetched on October 18 before merge preparation. The UI branch contains the 84 foundation commits plus the original prototype commit; the workspace, CI fix and documentation were published to both feature branches for [draft PR #1](https://github.com/unrealthind/Clawberry/pull/1). User explicitly requires draft status until every deployment gate passes. No merge, deployment, real credential enrollment, live external write or production acceptance is claimed here.


CI update at workspace head `202ac39`: the required Rust 1.92/Clippy/full-tiny-standard/build gate, frontend build/tests/format, desktop/phone browser checks, deployment-static/image checks, actionlint, npm audit and cargo-deny passed. The corrected RustSec audit ran and found six advisories in locked quinn-proto 0.11.13; all require at most 0.11.18. A targeted lockfile patch to 0.11.18 (and its required transitive dependencies) passes the full, tiny and standard locked/offline local Rust suites and release build, without suppressing advisories. Final patched-head CI remains required. PR #1 was draft at that checkpoint; consolidation is now complete, while deployment gates remain open.

## October 19 durable configuration

See [the active product plan](product-plan.md). Live global and owner settings use dedicated human GET/POST endpoints, optimistic revisions and schema-v8 storage. Profile/budget changes apply to subsequent inference; unchanged host credential sources remain outside model context. Agent soul/memory notes and SKILL.md instructions guide only that owner. Other skill files are saved as text and are not automatically installed or executed. Total instructions are bounded; settings cannot grant tools/secrets or alter Guard/TOTP/root policy.

The dev runtime has completed real Claude requests for all three configured profiles. Local environment credentials and runtime state remain git-ignored; actual provider verification does not establish live Guard, n8n writes or VM/service acceptance.

## October 19 thread delivery

Live decisions persist in schema v9 and use idempotent UUID-bound saves. Changed payloads or peer-owned thread IDs are rejected; models cannot record human decisions. File uploads accept nonempty files up to 4 MB in this view; backend file transfer retains its existing 5 MB bound. Downloads stay authenticated, owner-scoped and non-executable. Worker details expose structured findings/confidence/provenance and archived logs/artifacts; no memory is automatically promoted by inspection. Refresh controls surface partial failures, and ambiguous uploads require inspecting refreshed files before retrying.

Current validation: 43 browser passes and one intentional skip; 14 unit/API passes; full Rust 2,445 passes and five existing host ignores; paired and loopback smoke with stopped-state restore. Step 5 live deployment is still open; see [acceptance](deployment-acceptance.md).

## Live UI consistency and settings discovery

The connected layout now shares the prototype’s appearance cards, section icons/headings, card spacing, toolbar controls, input styling and save bar. The single App settings footer has a visible label, gear, global-preferences subtitle and selected state. Each agent retains a separate labeled gear with its own selected state and settings page. There is no duplicate global navigation entry.

App settings contains General, Claude & models, Notifications, Calendar and Connection & security. General includes appearance choices, Manage models and keyboard shortcuts. Agent settings includes Identity & memory, Skills, Credentials and Access & boundaries, with Open chat. Credentials explicitly states that references/enrollment are not connected; it does not show fictional credential records or accept keys. The header theme switch persists through the existing revision-checked global API; failed/conflicting saves show an error and refresh current preferences.

Navigation resets the content scroll to the top. The mobile drawer traps keyboard focus, closes with Escape, restores focus to its opener and is inert/hidden from assistive technology while closed. Thread-list scrolling keeps the global footer accessible. Local browser evidence: 45 passes, one intentional skip; frontend build/14 unit checks pass. These changes do not resume deployment acceptance.

## Legacy shell retirement

Both HTML entry points now import the current workspace. The old management application and global stylesheet are deleted. Approvals and Memory render inside the shared sidebar/header, and the settings approval shortcut navigates in-app. Watch schedules/recommendations are retained in Activity. Thread file transfers remain in the Focus Thread workspace. No separate management screen is needed to reach these features.

Write review keeps single-action/bounded-session distinctions, exact payload/hash binding, dedicated TOTP submission, immediate code clearing, expiry, remaining budgets, results, reconciliation and End session. Unconfirmed authorization disables blind resubmission until refreshed authority is read. Peer review binds each question or response hash. Memory keeps filters, pagination, confidence/provenance, edit/verify/stale/delete, exact sharing/revocation and owner isolation. These remain human operations using existing backend enforcement.

Local validation: 57 browser checks passed, one intentional skip; build and 14 unit/API tests passed. The running local gateway serves the same built application at both paths. These are local/fixture UI checks; live deployment and external-write acceptance remain deferred.

## Live chat polish

The current live chat uses a scrollable transcript and compact auto-growing composer with profile selection and the configured model name. Empty-state prompts fill a draft without sending it. Assistant messages have owner portraits and Copy/Copied feedback; copy failures explain manual selection. Reading earlier content disables automatic following and offers Jump to latest. The composer stays reachable in long desktop/phone conversations.

Sending shows the pending message, reply status, streamed text and Stop response. Stopping or losing confirmation retains the draft/partial text and requires history refresh before another send, because server work may still finish. Confirmed completion clears the sent draft and restores desktop focus. Phone Enter adds a newline; desktop Enter sends except during text composition, and Shift+Enter adds a newline. Drafts remain in tab memory only. Main-chat attachments and Markdown rendering are not implemented by this polish. Build/14 unit checks/67 browser checks pass, with one intentional skip; deployment acceptance stays deferred.

### Task execution evidence across chat turns (October 20, local)

The runtime now retains audited receipts for model-requested task creation, edits,
metadata and status operations. The next model turn receives the latest 12 receipts
for that same owner and conversation, including after a restart or a failed final
reply. These are historical execution evidence; `task.list` verifies current state
before a retraction or possible duplicate. Receipts stay outside visible chat and
semantic memory and do not expose other agents' or Focus Threads' results. Existing
conversations without receipts still require a task lookup; earlier assistant prose
alone does not establish whether an operation ran. No schema migration or execution
permission changes are needed. A rebuilt backend must be restarted to activate this
local change; browser refresh alone is insufficient.

## October 21 Phase 5 shared-memory controls

The live Memory workspace now lets the sharing owner edit, verify and mark shared
copies stale. Each operation binds the reviewed copy's current hash; verification
and stale marking show the exact fact and provenance before submission. Shared
edits use the existing human-only backend contract and leave the private source
memory unchanged. Both owners can inspect shared copies, while management controls
appear only for the sharing owner. These controls grant no peer communication or
external-write authority.

Validation for this local increment: frontend production build and all 16 unit
checks pass; the expanded shared-memory browser flow passes on desktop and phone
Chromium, including hash binding, private-copy preservation and peer control
visibility. Phase 5 remains in progress; these changes are uncommitted locally.

## October 21 Phase 5 app review

See [the review path and remaining gaps](phase5-review.md). Focus Threads now
separate Conversation from Files & evidence, retain tab drafts/previews, expose
summary editing and offer explicit authenticated text/raster previews. A composer
Files control opens thread evidence or explains creating a Focus Thread while
preserving the current draft. Text content enters chat only after the user chooses
Include text in message; image interpretation remains unconnected. Ambiguous
uploads require refreshing files before another upload.

Versioned closed cards render in completed assistant replies with readable
fallback for invalid/unknown blocks and owner-checked thread navigation. Cards
never execute operations. Tasks expose recorded history/worker sources on demand;
watch recommendations expose investigation/evidence/reservation correlation.
Approvals supports human-prepared peer questions and scoped responses with separate
exact reviews and closure/revocation. Pending review summaries exclude expiry and
include peer requests; configured daily budgets are shown independently of the
all-recorded usage totals. The normal New thread form and header refresh/recovery
entry points are restored. These changes preserve the existing security contracts.

## October 21 connection safety/reliability audit

See [the frontend/backend audit](frontend-backend-audit.md). Actual paired
browser → Vite → Rust contracts, dropped-response recovery and separate
loopback/paired restart/restore fixtures now supplement mocked browser coverage.
Core reads have runtime shape validation; network operations have deadlines;
SSE completion/size handling and private-response caching are hardened. The LAN
launcher uses a minimal GUI environment, rejects config symlinks before writing
and checks built login mode when skipping builds. Existing security enforcement
and deferred live-release gates remain intact.

The audit follow-up adds bounded task/thread pages with explicit Load more,
selected older-thread retrieval, complete settings/session-status/task-evidence
shape checks and important creation/authorization acknowledgement validation.
Private/shared memory expiry can be explicitly changed or cleared before separate
verification. The legacy list operations retain their 50-row cap; the earlier
all-owned-list statement confused internal database methods with API operations.
