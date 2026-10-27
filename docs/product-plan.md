# Prototype to daily-use product

Current delivery grouping: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance, including [connected operations](connected-operations.md) and the [source-aware foundation](source-aware-foundation.md). Phase 5 is next: app flows, attachments, evidence panels and rich cards, including shared editing controls and conversational peer review. Live integrations, external-write activation and production deployment remain gated/deferred. The earlier five-step order below retains useful feature and deployment evidence.

Reviewed October 20, 2026 (America/Edmonton). The eight-phase plan is the current delivery order. The five-step table below records the earlier checkpoint: local steps are complete and deployment remains deferred. PR #1 was subsequently merged; old draft/main-unchanged instructions no longer apply. Later Phase 0–4 work remains local-only until committed/pushed.

| Step | Deliverable | Acceptance | Status |
| --- | --- | --- | --- |
| 1 | Start local runtime with actual Claude routing | Loopback runtime runs; encrypted provider enrollment or environment credentials; actual provider turn recorded without exposing secrets | Complete locally; actual Fast/Normal/Deep Claude requests verified |
| 2 | Make connected data the normal workspace | Normal entry loads gateway data; empty/error states do not silently substitute samples; samples require explicit selection | Complete locally; desktop/phone checks passed |
| 3 | Persist global and agent settings in backend | Global preferences/model profiles and separate owner identity/memory/skill configuration survive browser/runtime restart; changes affect subsequent runtime turns; conflicts preserve drafts | Complete locally; persistence, conflicts, owner guidance and restart checks passed |
| 4 | Finish thread files, decisions and worker results | Upload/download, durable decisions and structured archived results integrated into thread workspace | Complete locally; uploads/downloads, durable idempotent decisions and structured archives verified |
| 5 | Validate approvals and always-on deployment | Guard, n8n/TOTP, replay/reconciliation, service recovery, backup/restore and target VM gates pass | Deferred by User; local approval/recovery checks pass, live deployment acceptance remains open |

Global settings own shared Claude configuration and Fast/Normal/Deep profiles. Each agent has one separate settings page. Development uses ANTHROPIC_API_KEY (or ANTHROPIC_OAUTH_TOKEN) from the host environment or git-ignored .env.local. Production web UI enrollment requires a dedicated encrypted backend endpoint and is planned before production acceptance; it is not implemented or enabled in this slice. No credential belongs in chat, browser storage, operation logs or model-visible configuration. Vault enrollment for integration secrets stays disabled. Settings cannot grant execution privileges, alter root policy, bypass TOTP, or cross Janet/Harley boundaries.

See [current UI scope](ui-workspace.md), [merge gates](merge-readiness.md), [phase ledger](p06-p20-progress.md) and [worklog](../worklogs.md) for evidence. Local tests/fixture providers do not prove actual Claude or deployment acceptance. Historical documents remain historical; this file defines the current order.

## October 19 local implementation

- `python3 scripts/dev-local.py --build` starts the real loopback runtime using git-ignored `.runtime/` state and `.env.local` (mode 0600). An existing nonempty shell environment credential takes precedence. Values are not printed or passed in process arguments. The runtime is currently running at 127.0.0.1:8080.
- `python3 scripts/verify-local.py` creates an owned verification thread and makes small real Fast/Normal/Deep requests. All three completed with the user-provided environment credential. Runtime status reports verification from successful usage records for current provider/model mappings; it is not an uptime guarantee.
- `/prototype.html` uses connected data by default. `?mode=sample` explicitly opens the fictional browser preview. A gateway outage does not switch to fictional data.
- Human-only `GET/POST /api/settings/global` and `/api/agents/:agent/settings` store nonsecret versioned settings in schema-v8 `system.db`. Stale revisions return 409 without overwriting the draft. Settings values do not enter operation logs; audit entries contain scope/revision only.
- Global appearance/notification/calendar preferences and shared Claude profiles/budgets are durable. Next requests load saved routing. Agent settings persist soul.md, memory.md notes, default reasoning profile, skill metadata and text files; guidance is included only in that owner's subsequent turns. Skill scripts do not execute. Policy/tool/secret permissions remain administrator-owned.
- Root `/` is now the primary current workspace, with `/prototype.html` as an alias; approvals and memory are integrated; thread files, decisions and worker archives are connected. Production credential enrollment remains unfinished.
- Back up complete stopped schema-v9 state before upgrading an existing deployment. Schema v8 and older binaries cannot open the upgraded state; restore a matching full backup for rollback.

Validation: frontend production build and 14 unit/API checks pass; 41 desktop/phone browser checks passed with one intentional skip. Full locked/offline all-target Rust suite: 2,442 passed, five existing host-dependent sandbox tests ignored. Paired and token-free fixture gateway smoke passed, including saved guidance/default-profile dispatch and stale revision rejection. After restarting the real local runtime, health, settings, built prototype and all three retained provider verification statuses were confirmed. Final-head CI/security remains a separate merge gate.

Tiny and standard locked/offline suites also pass: 1,716 executions each, five existing host sandbox ignores. Local Rust 1.99 Clippy still reports inherited newer-toolchain diagnostics; no new settings-module diagnostics remain. Required Rust 1.92 CI is authoritative and must pass on the published head.

## Thread completion and deployment validation

Step 4 is complete locally. The live Focus Thread workspace groups linked tasks, file contents via authenticated upload/download, human-recorded decisions and structured archived worker results. Decisions use caller-generated UUIDs: retrying the exact ID/content is idempotent, while changing content or owner/thread scope is rejected. Only the authenticated human API may record them; a decision never grants execution approval. Schema v9 adds the decision ledger and preserves prior settings/data.

Worker inspection shows summary, findings, confidence, source references and failures, with downloads of the transcript, tool log, shell log, sources, costs, memory candidates and generated artifacts. Unrelated workers are excluded from the thread. Empty/loading/error states and desktop/phone behavior are verified. Samples remain explicit.

Step 5 is **deferred by User**, not complete: local policy/approval/replay/reconciliation and recovery tests pass, including a new independent-connection concurrency check, but live deployment requires the confirmed target and existing Guard/n8n services. The Linux service-unit newline bug and previous-head CI configuration lint are fixed. See the [acceptance procedure and outstanding inputs](deployment-acceptance.md). Production credential enrollment and live controlled writes remain disabled/unperformed.

Validation for this delivery: 2,445 full Rust tests passed, five existing host sandbox checks ignored; frontend production build and 14 unit/API tests passed; 43 desktop/phone browser checks passed, one intentional skip. Both authenticated gateway modes passed the extended fixture smoke and complete stopped-state backup/restore. Local Rust 1.99 retains inherited Clippy diagnostics; required Rust 1.92 CI must pass on the published head. Main was unchanged and PR #1 was draft at that historical checkpoint; consolidation is now complete.

Tiny and standard feature suites each passed 1,719 tests with the same five host sandbox ignores. The v8-to-v9 migration regression preserves saved global/agent settings. The real local runtime was stopped, its complete `.runtime` tree backed up privately with SQLite integrity checks, and restarted successfully on schema v9. Health, built UI, both owner settings and the decisions endpoint returned 200; existing Fast/Normal/Deep verification records remained intact. No new billable Claude turn was needed.

User chose to put deployment acceptance on hold because it is too advanced for the current local-development stage. Do not continue target-host setup or live integration acceptance until User resumes it. No deployment details are currently being requested.

## Local UI refinement — October 19

User’s screenshot showed that the connected implementation had drifted from the intended prototype and that global settings was hard to find. This follow-up has its own UI priorities; the deployment step 5 above stays deferred.

| UI priority | Deliverable | Status |
| --- | --- | --- |
| UI-1 | Restore consistent layout, spacing, buttons and visible controls across connected views | Complete locally; live toolbar, forms, messages, activity cards and settings use the established design |
| UI-2 | Make the single global App settings entry clear, keeping agent settings separate | Complete locally; labeled footer with gear/current state, owner gear controls, consistent settings navigation |
| UI-3 | Further chat/composer polish and attachment controls | Chat/composer polished locally; main-chat attachment controls remain planned |
| UI-4 | Unified Files/Activity/Approvals navigation inside one workspace | Approvals, Memory and Activity/watch controls unified; global Files view remains planned |
| UI-5 | Dedicated thread tabs/panels for conversation and evidence | Planned |
| UI-6 | Broader audit of remaining controls and unfinished features | Planned |

App settings restores appearance cards, the Manage models shortcut, keyboard shortcuts and Connection & security. Agent settings exposes Identity & memory, Skills, Credentials and Access & boundaries together; Credentials is clearly read-only/unconnected and accepts no secrets. The header theme switch saves through the versioned backend and reports conflicts without overwriting newer preferences. Opening another view resets the content scroll position so its title and controls are visible. The mobile drawer keeps keyboard focus within it while open, restores focus on close and is inert while hidden.

Validation: frontend production build and 14 unit/API tests pass; desktop/phone browser suite passes 45 checks with one intentional skip. Browser checks cover one visible App settings entry, gear/current state, separate agent setup, appearance persistence/conflicts, settings navigation, scroll reset, mobile focus and horizontal overflow. Deployment remains deferred; PR stays draft and main unchanged.

## Retire the duplicate management UI

User asked why the unfinished old UI was still present. The duplicate root shell and its stylesheet are removed. Both `/` and `/prototype.html` now load the current application, with in-app Approvals and Memory and watch controls retained under Activity. Settings no longer links out to `/`. Disconnect stays in the current application and offers Connect; explicit sample preview remains optional.

Existing security/memory operations are retained, including dedicated TOTP transport, exact review hashes, session budgets/end/reconciliation, private/shared memory review and disabled-by-default watch settings. This work consolidates local product UI; it does not resume deployment step 5. Local Fast/Normal/Deep remain configured for Sonnet 5.5 during development. Build, 14 unit/API tests and 57 desktop/phone browser checks pass (one intentional skip); final-head CI remains separate.

## Live chat polish — October 19

The conversation now has its own scroll area above a compact, growing composer. Profile selection and the configured model name share the composer toolbar; saved runtime routing remains authoritative. Replies use owner portraits, readable spacing and explicit Copy/Copied feedback. Empty chats offer draft-only prompts. Long conversations keep the composer reachable and expose Jump to latest when reading earlier messages. Earlier-history loading preserves the reading position.

Desktop Enter sends and Shift+Enter adds a newline, with composition input protected. Touch Enter adds a newline and the visible Send control submits. Sending shows an immediate user message and reply status, with Stop response available. Interrupted or stopped requests retain the draft and received partial text, show inline recovery guidance and block resubmission until history is refreshed; Stop does not promise server-side cancellation. Desktop input focus returns after confirmed completion. No drafts, replies or credentials are added to browser storage.

Build and 14 unit/API checks pass; 67 desktop/phone browser checks pass with one intentional desktop skip. No backend/security contract changed, no billable provider turn was needed, and deployment remains deferred. Main-chat attachment/image understanding and rich Markdown rendering remain future work.
