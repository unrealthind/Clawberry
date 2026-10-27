# P06–P20 implementation and acceptance ledger

October 20 Phase 4 source-aware local foundation is complete: owned audited expiry review, validated selective promotion, exact-reviewed shared-copy management, identity/expiry-bound peer grants and quiet-hour watch recovery with evidence-linked notifications. Full/tiny/standard suites pass 2,494/1,768/1,768; frontend/browser regressions and local authenticated fixture flows pass. See [source-aware foundation](source-aware-foundation.md). This local milestone does not enable schedules, peer model capabilities or writes, and does not close the broader P13–P15 live-source/conversational gates below. Phase 5 app completion is next.

October 20 Phase 3 local client/runtime acceptance is complete: read correlation survives projection; reviewed write contracts require explicit operations; manifest hashes use canonical JSON; gateway action/audit/execution IDs are separate; authorization failures burn reviews and clean partial bearers; known secret/code echoes cannot become persisted metadata; closure evidence survives restart and the UI distinguishes local revocation from unconfirmed remote closure. Final full/tiny/standard suites pass 2,481/1,755/1,755, with debug/release/runtime and frontend/browser checks passing. `writes.db` now has schema version 1; matching stopped-state backup/rollback and updated administrator contracts are required. See [connected operations](connected-operations.md). These local fixtures do not close P11–P12's live gateway/TOTP/service gates or enable external writes.

October 20 local acceptance: User completed manual host setup. The eight-phase Phase 2 local milestone is now complete: real rootless isolation, cgroup limits, read-only inputs/root, timeout/output limits, media, offline packages, orphan cleanup, broker single-use/reopen and protected default-deny host-policy checks pass with SELinux enforcing. See the exact scope/evidence in [the eight-phase plan](eight-phase-plan.md). Original P06–P09 deployed/live-provider gates below remain open; no execution capabilities, live integrations or service were enabled. All current hardening changes remain local-only until committed/pushed.

October 19 Rocky development follow-up: the current [eight-phase plan](eight-phase-plan.md) groups P01–P05 as Phase 1 and P06–P09 as Phase 2. Phase 1 has fresh passing ownership/persistence/work integration checks. Phase 2 adds strict completed Guard transport validation, canonical bounded execution scope, safer-alternative re-admission coverage, expanded encoded-secret redaction and UTF-8-safe sandbox output limits. Real rootless sandbox/host-policy acceptance is still pending on this VM; use `dev/phase2-acceptance.sh` with a reviewed local image. This does not resume production deployment or close the live HF/S14, n8n/TOTP or service gates. The earlier draft-PR instructions below are historical; PR #1 is merged and main contains the consolidated implementation.

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

Reviewed 2026-10-18, America/Edmonton. See [P06–P12 runtime integration](p06-p12-runtime.md) for the newly connected paths and VM evidence. **This milestone is not complete.** Do not enable execution or external writes on the strength of component tests.

## October 16 coding checkpoint — paused at User's request

This is a partial delivery toward P20, not phase completion. Deployment remains deferred until coding is ready. See [checkpoint APIs, configuration and remaining work](coding-checkpoint-2026-10-16.md).

- The remote branch already contains opt-in guarded execution, media, reviewed read workflows, deep reviews and dedicated write-authorization service/UI. Those newer components are preserved; see P06–P12 below for their recorded acceptance limits.
- P13: independent human-configured read-only watch schedules, UTC quiet hours, atomic claims, daily limits, owned task/failed-read evidence, restricted investigators and durable recommendations/failure notifications. Defaults remain disabled. External-source monitoring and richer notification routing remain unfinished.
- P14/P20: private-memory pagination/server filters and older-fact retrieval; exact human-reviewed ordinary/current fact promotion into separate `data/shared.db`; relevant shared context and audited revocation. Shared editing/verification and automatic review policies remain unfinished.
- P16: authenticated SSE chat (provider deltas when supported, single-result fallback otherwise), internal-payload suppression, Stop/disconnect semantics, atomic final chat commits and retained uncertain-cost reservations. Optional `gateway.app_dir` serves the built app. Main-chat attachments/images, robust reconnect and browser acceptance remain unfinished.
- Human main/thread history pages are independent of the bounded 50-message model context.

Schema version is now **7**. Back up the complete stopped state tree, including shared/private databases, before upgrade; rollback requires the matching backup. Integrated execution/write acceptance and full thread/card/memory completion remain unfinished. No live external write was performed in this checkpoint.

| Phase | Implemented locally | Still required for acceptance |
| --- | --- | --- |
| P06 | Root-owned exact policy; immutable denies; authenticated human approval/revocation operations; atomic single-use consumption and replay protection; real VM default-deny policy loads | Enabled host/task path acceptance; positive/negative policy and protected-audit deployment acceptance |
| P07 | Credential-broker-backed HF client; strict/fail-closed verdicts; automatic deep-profile reviews for shell/read/write blocks; safer scope validation; exact recorded-review + human exception grants; live Guard BLOCK/S14 and Anthropic review/exception path verified; validated category-only audit | Diagnose read-only S14 classification; live SAFE/unavailable/safer-action coverage and full service acceptance |
| P08 | Encrypted private grants; hidden terminal registration; scoped injection/rotation; known base64/hex redaction; reviewed read-result projection; expiring/revoked encrypted session bearers; identifier-only usage audit | Live integration schemas and service credential setup; deployed rotation/restart acceptance; additional adversarial encoded-output coverage |
| P09 | Guarded task-owned rootless Podman path; digest-pinned local image; no host fallback/network; read-only inputs/root; bounded tmpfs/output/process limits; offline package operations; cancellation-safe cleanup. Real VM isolation/timeout tests pass | Enabled end-to-end broker acceptance; administrator-bundled wheel install acceptance; persistent custom packages; worker grants; restart orphan-container recovery |
| P10 | Owned authenticated uploads/downloads; checksums; guarded image previews and short-video filmstrips; task/thread-linked artifacts with provenance; disposable staging. Real decoder tests pass | Enabled end-to-end admission/artifact/UI acceptance; larger media and bounded n8n attachment ingestion; lifecycle/cleanup recovery |
| P11 | Protected reviewed read contracts; CLI/gateway/model opt-in wiring; scoped fields/resources; Guard/deep-review admission; credential injection; allowlisted response projection; normalized body/HTTP failures; existing email response compatibility; live bounded read and n8n execution ID correlation; temporary gateway restart passed | Full service deployment; broader read/polling/error recovery acceptance; attachment ingestion and separate gateway audit ID |
| P12 | Dedicated human TOTP endpoint/UI; immutable prepared reviews; strict gateway authorization attestation; encrypted token expiry/removal; durable scoped budgets/replay ledger; guarded dispatch; status/results/End session; terminal evidence reconciliation | Existing gateway adapter and live TOTP tests; shared transactional gateway replay/budgets/restart/idempotency; browser acceptance. No live external writes performed |
| P13 | Independent read-only watch schedules, UTC quiet hours, atomic claims, budgets and restricted investigation/recommendation workers; quiet-hour recovery and source-linked notifications | External-source monitoring, richer notification routing and live-provider acceptance; local fixture end-to-end evidence is tracked in Phase 4 |
| P14 | Private semantic-memory CRUD/retrieval; complete pagination/server filters; selective validated promotion; owned automatic expiry review; exact-reviewed shared copies, edits/verification/staleness and audited revocation | Broader source coverage and independent source re-verification; shared editing controls belong to Phase 5 |
| P15 | Durable exact-question/exact-response exchange; human-only identity/expiry/hash-bound review; expiry/revocation; one response; automatic scope closure; retained history; no peer DB access; legacy grants fail closed | Model capability enablement after security gates and full conversational app flow; local human management contract is covered |
| P16 | Chat-first app; in-memory paired token; owner-scoped queries; SSE/provider deltas with fallback; Stop/disconnect and uncertain-cost semantics; history pagination; optional static serving; bounded thread files | Main-chat attachments/image understanding; robust reconnect; browser/mobile acceptance |
| P17 | Thread list/create/open; separate conversations; summaries; thread files; owned API operations | Integrated thread tasks/artifacts/activity and main-chat reference cards; browser acceptance (full human history pagination now exists) |
| P18 | Versioned closed card schema/renderer and invalid/executable payload rejection tests | Runtime card envelopes; remaining card types; supported-action integration and chat fallback; browser tests |
| P19 | Owner activity/audit and workers; exact peer question/response review UI; confirmation bound to current payload and review state; dedicated TOTP single/session review, status and End session controls | Integrated pending security approvals; deployed session results/uncertainty/budgets/End session and browser acceptance |
| P20 | Full private/shared listing pagination; server source/confidence filters; provenance; private edit/delete/verify/stale; exact-reviewed shared promotion and revocation; audited changes | Shared editing/verification; UI interaction/accessibility/browser tests |

## n8n reference contracts

User supplied examples of IMAP management, exact-email SMTP authorization, and read-only IMAP workflows. They establish the **integration pattern**, not a universal email-only product API. User reports the SMTP workflow works; live behavior was not exercised in this session.

- IMAP management: request manifest → TOTP authorize → allowlisted/budgeted action → end; maximum five-minute session. Purpose text is not mailbox/UID/agent/task scope. The runtime must enforce the reviewed resource scope itself and the deployed gateway must meet the transactional/replay acceptance criteria.
- SMTP: request exact draft → TOTP authorize → execute the server-locked draft with an empty action body → end; one email. The supplied export contains proposed attachment changes. Attachment limits and exact deployed revision still need acceptance evidence; do not infer approval of every proposed capability from a working text send.
- Read-only IMAP: named operations include search/read/list and attachment discovery/download, with server-derived Message-ID/reply metadata. The runtime read adapter is independently bounded to 1 MiB JSON results; the example's larger base64 attachments need a separate artifact ingestion path, not injection into model context.
- Workflow-static authorization data, rolling audit windows, and separate replay counters are not sufficient evidence of safe concurrency/restarts/shared-secret replay protection. Do not turn a `retry_allowed` flag into replay permission. A failed verification or timeout can follow a successful external write.
- Gateway instructions saying an agent should receive/pass TOTP are **not** the product contract. Codes must travel through a dedicated authenticated human flow, outside model context, operation logs, analytics, and artifacts. Tokens remain broker-owned.

No supplied workflow export, credential, or raw email content was copied into the repository. Live bounded read-only n8n calls succeeded through user-approved Guard exceptions. No email write was made.

## Deployment and RAM scope

User's current target is **one Proxmox VM, four cores, 6 GB RAM**, replacing the original nested LXC target. Janet and Harley remain logically separate within one runtime. Identity does not depend on VM/container placement.

The core uses remote model providers; no local Guard or reasoning-model weights are loaded. Model workers are capped at two globally and one per permanent agent, using durable active leases. Context, memory results, HTTP results, uploads and credential manifests are bounded. These are limits, not a measured production RSS claim. VM acceptance must measure idle/peak RSS and verify sandbox cgroup limits.

The React app builds to static assets. Vite/Node is a development/build tool; a permanent Node development server is not the production deployment plan. Optional same-origin static serving is now implemented through `gateway.app_dir`; authenticated TLS/SSH-tunnel deployment still needs acceptance.

## Verification limits

Local component and integration suites are recorded in `worklogs.md`. No connected browser was available; a native Safari attempt was denied computer-use permission. Therefore visual and browser interaction acceptance is **not** claimed. VM access and rootless sandbox tests passed. Live Guard BLOCK/deep-review/exact-exception and bounded n8n email reads with execution correlation passed. Autonomous SAFE-path, deployed writes and full service acceptance remain open.

Execution/write broker capabilities are now opt-in through a root-owned closed per-agent configuration, whose defaults remain empty. The VM explicitly enables Janet’s reviewed n8n.read contract; external writes and shell/media execution remain disabled. Cross-agent model capabilities remain disabled. Read-only watch schedules are separately configurable and default off. The authenticated human API can manage owned internal records and reviewed peer messages; that does not authorize tools or external writes.

## Latest acceptance (2026-10-06)

All 1,179 library tests and strict all-target Clippy passed before delivery.
Anthropic transport now omits unsupported temperature settings and parses mixed
text/non-text content. Deep reviews require a complete typed JSON decision.
Exact approvals remain spent across DB/process restart. Autonomous read admission
now atomically records request IDs, including protection when Guard verdict changes.

Live Guard reports S14 (Code Interpreter Abuse) for the bounded email read; the
category is known but its cause remains unresolved. Do not infer this is an approved
classification change or bypass. Two temporary localhost gateway starts passed;
there is no installed user systemd unit, so always-on service acceptance is pending.
The runtime links review, human approval, dispatch, result and n8n execution ID.
No private email content or supplied workflow exports are retained in this ledger.
All write workflows currently share a TOTP secret according to User; cross-workflow
transactional replay protection remains a requirement before writes can be enabled.

### October 18 UI workspace and draft delivery

Development resumed at User’s request; the UI branch contains the foundation history. The current workspace supports sample task create/edit/prioritize/complete/reopen, calendar deadlines, and Focus Thread context for tasks, filenames, decisions and fictional handoffs. Opt-in same-origin pairing connects real SSE chat, owned internal tasks and read-only activity. `task.edit` validates ownership/inputs/terminal state and commits fields plus event atomically.

Global settings live once in the User footer: shared Claude API/model profiles, model/budget drafts and workspace preferences. Agent settings live beside Janet/Harley: consolidated identity/personality + memory source files, skills, credential previews and boundaries. No duplicate personality form or standalone Skills/Vault workspace entry remains. Drafts do not activate live provider/model/permission changes; secret entry is disabled. See [complete UI scope](ui-workspace.md).

Local frontend build and 13 unit/API checks pass. Full/tiny/standard locked offline Rust suites pass using local Rust 1.99; the locked/offline release build and browser suite also pass (37 checks, one intentional desktop skip). Final-head Rust 1.92 CI and security jobs remain required. Browser testing exposed model/budget accessible-name issues and ambiguous hidden-editor test selectors; these were corrected, including keeping navigation names stable while a draft is dirty. The previous cargo-audit CI command’s unsupported `--locked` option was replaced with explicit `--file Cargo.lock`; no vulnerability suppression was added.

User explicitly chose to retain draft status until all deployment gates pass. No main merge, deployment, live credential enrollment, external-calendar synchronization or external-write acceptance is claimed. P06–P20 remain incomplete.


CI update at workspace head `202ac39`: the required Rust 1.92/Clippy/full-tiny-standard/build gate, frontend build/tests/format, desktop/phone browser checks, deployment-static/image checks, actionlint, npm audit and cargo-deny passed. The corrected RustSec audit ran and found six advisories in locked quinn-proto 0.11.13; all require at most 0.11.18. A targeted lockfile patch to 0.11.18 (and its required transitive dependencies) passes the full, tiny and standard locked/offline local Rust suites and release build, without suppressing advisories. Final patched-head CI remains required. PR #1 was draft at that checkpoint; consolidation is now complete, while deployment gates remain open.


## October 19 — local product steps 1–3

Local actual Claude Fast/Normal/Deep routing is verified, the prototype defaults to live gateway data, and schema-v8 backend settings persist global preferences/profiles/budgets and separate owner identity/memory notes/skill text. Saves use revision conflicts; subsequent runtime requests use saved guidance/routing without granting new capabilities. Development uses environment credentials; production web enrollment remains planned. Thread file/decision/result completion is next, followed by unchanged Guard/n8n/TOTP/always-on deployment acceptance. This does not mark all P06–P20 complete. See [the product plan](product-plan.md).


## October 19 thread and recovery delivery

Step 4 is complete locally: authenticated thread file upload/download, schema-v9 durable human decisions with idempotent retries, and structured worker results/archive downloads. Local validation: 2,445 Rust tests and 14 frontend checks passed; 43 browser checks passed with one intentional skip; five existing host sandbox Rust tests remain ignored. Both gateway modes passed complete stopped fixture-state backup/restore. The Linux service unit newline bug is fixed, and concurrent write-budget admission is tested across independent connections.

Step 5 is deferred by User; target VM, live Guard/n8n/TOTP and always-on acceptance remain open. Production credential enrollment remains unfinished. Final-head CI is required; At that historical checkpoint PR #1 was draft and main was unchanged; the later authorized consolidation superseded that restriction. See [deployment acceptance](deployment-acceptance.md).
