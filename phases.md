# Implementation Phases

User's current eight-phase numbering is maintained in [eight-phase delivery plan](docs/eight-phase-plan.md). The P00–P25 identifiers below retain the original detailed roadmap numbering.

Current delivery status — October 20, 2026: [eight-phase plan](docs/eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

Last reviewed: 2026-10-20 (America/Edmonton).

This is the working status tracker for [ROADMAP.md](ROADMAP.md). It does not replace the detailed roadmap or the constraints in [AGENTS.md](AGENTS.md). Session evidence and review findings live in [worklogs.md](worklogs.md).

## Current state

October 20 source-aware foundation update: Phase 4 of the current eight-phase plan is complete locally. Shared-memory edits/verification/staleness are owner/hash-bound and human-only; selective promotion uses the common sensitive/credential validator; expired private facts are automatically marked stale and audited without deletion. Peer grants bind identity/expiry and reject legacy bindings for new delivery. Watches finish/recover during quiet hours and retain source-linked notifications. Full/tiny/standard suites pass 2,494/1,768/1,768 executions; strict Clippy, debug/release builds, local daemon recovery and frontend/browser regressions pass. See `docs/source-aware-foundation.md` for scope. Original P13–P15 broader live/conversational gates remain open; production schedules, peer model capabilities and external writes remain unenabled. Work remains local-only.

October 20 connected-operations update: Phase 3 of the current eight-phase plan is complete for local client/runtime acceptance. Final full/tiny/standard suites pass 2,481/1,755/1,755 executions; strict Clippy, debug/release builds, runtime recovery, frontend and desktop/phone browser checks pass. Added explicit workflow operation allowlists, canonical manifest hashing, separate durable audit/execution correlation, versioned write-ledger migration, secret-safe authorization metadata, terminal authorization failures and durable remote-closure confirmation. Original P11–P12 live gateway/TOTP/service gates remain open; external writes and capabilities remain disabled. See `docs/connected-operations.md` and the eight-phase plan for scope and upgrade requirements. Work remains local-only.

October 20: Phases 1 and 2 of User's eight-phase plan are complete for local development acceptance. Seven distinct opt-in host/container checks pass on Rocky with rootless Podman and SELinux enforcing, including real resource/isolation boundaries, offline packages, media, orphan cleanup, single-use broker admission and protected host policy. Final full regression suite passes 2,457 executions; opt-in checks were separately executed successfully. The older detailed P06–P09 production/live-provider gates remain open, and no capabilities/service/external writes were enabled. See `docs/eight-phase-plan.md`. This work is local-only; fetched main matches local HEAD and uncommitted changes are preserved.

October 19 Rocky development update: User authorized consolidation into main; both redundant feature branches were removed after their shared tip was included. PR #1 is merged. Earlier instructions in this tracker to keep the PR draft/main unchanged are historical. This Rocky Proxmox VM is the development machine; deployment acceptance remains deferred. Current VM setup and repeatable checks are documented in [Rocky development](docs/rocky-development.md).

October 18: development resumed for the UI workspace and merge preparation at User’s request. The workspace now includes task management, calendar deadlines, thread context, global app settings, consolidated per-agent setup, and opt-in connected chat/tasks/activity. P06–P20 remain incomplete; UI preview work does not close deployed security/write or runtime-integration gates. See [current UI scope](docs/ui-workspace.md), [acceptance ledger](docs/p06-p20-progress.md), and [merge preparation](docs/merge-readiness.md). The October 16 checkpoint remains a historical record.

### Historical implementation checkpoints

The following paragraphs retain the October 18/earlier source and deployment evidence. The October 19 consolidation and October 20 local acceptance above supersede their branch/host status; do not interpret them as current activation instructions.

The supplied [GitHub source](https://github.com/unrealthind/Clawberry) is now imported, preserving the product documents and inherited guide. Planning commit `6697f6b` was pushed to `main`; core implementation is on `feat/multi-agent-foundation`; `feat/ui-workspace` includes that foundation plus the UI work. GitHub was checked on October 18: main is still at `6697f6b`, the UI branch at `b04559f` before this session’s uncommitted follow-up. The follow-up is published through [draft PR #1](https://github.com/unrealthind/Clawberry/pull/1), with both feature branches consolidated. User explicitly requires all deployment gates to pass before merging; main remains unchanged.

P00–P05 are complete for the authorized local core, through the model router. P00's inherited formatting, Clippy, and tiny feature-gating issues are resolved; the baseline operational inventory and offline TUI/daemon checks are complete. User explicitly deferred LXC creation/deployment.

Janet/Harley now have isolated histories/workspaces, durable tasks and Focus Threads, scoped temporary model workers, inspectable archives, selective memory candidates, and configurable fast/normal/deep inference with atomic budgets. Full, tiny, and standard suites passed; evidence and counts are in [worklogs.md](worklogs.md). See [core usage](docs/core-foundation.md). Execution and write adapters are implemented behind administrator opt-in configuration. Janet’s reviewed read-only email contract is enabled and verified through exact human Guard exceptions; shell/media execution and external writes remain disabled; cross-agent model communication and autonomous heartbeats remain disabled.

Use these statuses: **not started**, **in progress**, **blocked**, **complete**. A missing implementation is not automatically a blocked phase. Completion requires the roadmap exit criteria and recorded evidence.

P06–P12 now have connected guarded runtime, n8n authorization and approval UI paths, with real rootless VM isolation/media tests. Live HF Guard, Anthropic deep review, exact human exceptions, bounded email reads, n8n execution correlation and temporary localhost gateway restart checks have passed. They remain in progress pending Guard S14 investigation, autonomous SAFE-path/full service/write acceptance and the limitations in [runtime integration](docs/p06-p12-runtime.md). P14–P20 have substantial local component work but are not complete. See the [implementation and acceptance ledger](docs/p06-p20-progress.md) for implemented scope and missing gates. User's deployment target is now one Proxmox VM with four cores and 6 GB RAM; VM access is confirmed (Debian 13, four CPUs, approximately 6 GB RAM); rootless sandbox acceptance passed. The always-on systemd service is not installed; full runtime/service deployment acceptance remains pending. Supplied n8n exports are examples for reviewed workflow adapters, not a universal email-only API. No external writes are enabled.

## Phase tracker

IDs below follow `ROADMAP.md`'s detailed numbering. Dependencies are recommended execution gates; they do not imply the existing roadmap has been rewritten.

| ID | Phase | Status | Dependencies / execution gate | Evidence required to close |
| --- | --- | --- | --- | --- |
| P00 | Fork, Baseline, and Inventory | Complete (Rocky development VM) | Source/build/inventory/quality verified; production deployment deferred by User | Fresh Rust 1.92 full/tiny/standard tests, fmt/Clippy, debug/release builds, daemon/recovery/TUI and frontend/browser checks passed; see `docs/baseline.md` |
| P01 | Permanent Multi-Agent Foundation | Complete (local core) | Baseline and ownership/authentication/audit foundations verified; no execution capabilities enabled | Isolation/persistence tests and one-daemon mock-provider smoke test passed; see `docs/permanent-agents.md` |
| P02 | Tasks + Focus Threads | Complete | P01 | Owned persistent work, dependencies/events/files, compact summaries, separate histories and restart/migration tests |
| P03 | Temporary Subagents | Complete (restricted workers) | P01–P02; execution awaits P06–P09 | Selected context/profile/budget, status/wait/cancel/retry, bounded completion, archive and ephemeral cleanup verified |
| P04 | Task Archive + Memory Candidate Pipeline | Complete | P03; general semantic memory remains P14 | Inspectable success/failure archives; source/confidence-aware selective promotion/rejection and sensitive/owner denial tests |
| P05 | Model Router | Complete | P00–P01 | Configurable bounded fast/normal/deep inference; explicit escalation, token/cost ledger, concurrent budget and uncertainty checks |
| P06 | Root of Trust + Policy Engine | In progress | P01 identity/audit foundations | Host-owned non-writable policy; hard-deny tests; authenticated scoped approval state; audited denials |
| P07 | Llama Guard + Opus Review | In progress | P05–P06 | Safe/block/error paths; unavailable Guard blocks gated execution; safer alternatives rechecked; no silent override |
| P08 | Secret Broker | In progress | P06; P07 before live gated tools | Agent/tool capability checks; encrypted storage; injection/redaction tests; no secrets in model context or logs |
| P09 | Sandboxed Shell | In progress | P06–P08 | Selected isolation mechanism; filesystem/network/resource limits; logged package operations; escape/access denial tests |
| P10 | Media/File Workflows | In progress | P03–P04, P09 | Uploaded-file/video workflow; scoped artifacts with provenance; resource limits; approved temporary-file cleanup |
| P11 | n8n Bridge | In progress | P05–P08; read-only until P12 | Existing gateway contract inventory; authenticated reads; broker-held credentials/tokens; agent/task/request/session/action/execution correlation; normalized errors and reconciliation |
| P12 | TOTP Write Boundary | In progress | P06–P08, P11 | Single-action exact payload binding and bounded manifest/resource/session scope; n8n/TOTP only; atomic replay/budget enforcement; expiry/revocation; code/token exclusion; idempotency/recovery checks |
| P13 | Janet and Harley Heartbeats | In progress | P02–P05, P06–P12 | Independent schedules/quiet hours/budgets; investigate before notify; scoped workers; no autonomous external writes |
| P14 | Long-Term Memory | In progress | P01, P04, P06–P08 | Separate domains; confidence/provenance; sensitive-category exclusions; inspect/edit/delete; explicit shared promotion |
| P15 | Cross-Boundary Approval | In progress | P06–P08, P14 | Default deny from P01 onward; exact scoped exchange approved by User; expiry/closure; no direct private DB access |
| P16 | Chat-First App Foundation | In progress | P01–P02; authenticated gateway; gated operations use their completed backend phases | Streaming main chats; files/images; reconnect; responsive layout; ownership checks on API/events/uploads |
| P17 | Focus Threads UI | In progress | P02, P16 | Thread conversations/tasks/artifacts; compact main-chat references; full history retrievable; no silent archival |
| P18 | Rich Card System | In progress | P16; corresponding backend features | Versioned schemas; safe invalid-payload handling; supported actions only; chat fallback |
| P19 | Activity and Approvals | In progress | P06–P07, P12, P15–P18 | Pending decisions/workers/failures visible; single-action vs session review; expiry/budgets/End session/results; authenticated decisions and server-side scope enforcement |
| P20 | Memory Management UI | In progress | P14, P16 | Search/filter/inspect/edit/delete/verify; source/confidence/sensitivity visible; audited changes |
| P21 | Maintenance Sessions | Not started | P02, P04, P14, P17–P20 | Review agenda; explicit User decisions; audited lifecycle/retention changes |
| P22 | Packaging and Desktop App | Not started | P16–P21; remote session security already established | Desktop package; secure LXC connection; notifications/deep links; reconnect; browser remains usable |
| P23 | Mobile-Friendly Companion | Not started | P16–P21; P22 not required | Phone/PWA chat/uploads/approvals/TOTP tested; all critical flows usable on mobile |
| P24 | Reliability / Hardening | Not started | All enabled v1 capabilities; foundations begin earlier | Restore and upgrade rehearsal; restart recovery; quotas/rate limits; cleanup; reconciliation; health monitoring |
| P25 | Voice Preparation | Not started | Event/modality contract starts P01–P02; streaming foundation P16 | Compatible audio/event metadata, cancellation, chunking contracts; no full voice feature required |

## Mapping to AGENTS.md

`AGENTS.md` groups the same direction into ten broad phases. Use P00–P25 for work entries to avoid ambiguity.

| Broad phase in AGENTS.md | Detailed roadmap phases |
| --- | --- |
| 0 — Fork Baseline | P00 |
| 1 — Multi-Agent Foundation | P01–P02 |
| 2 — Subagents | P03–P04; full semantic promotion completed in P14 |
| 3 — Security + Secrets | P05–P08, P15; audit foundations start P01 |
| 4 — Sandboxed Execution | P09–P10 |
| 5 — n8n Integration | P11–P12 |
| 6 — Heartbeats + Proactivity | P13 |
| 7 — Memory | P14, P20 |
| 8 — App | P16–P20, P22–P23 |
| 9 — Maintenance + Quality | P21, P24 |
| Later — Voice and integrations | P25 preparation; full voice/home/phone integrations remain later work |

## Delivery gates

These gates preserve the product's security requirements while allowing incremental implementation.

1. **Baseline:** Capture upstream revision, unchanged behavior, tool entry points, and schema before selecting replacements or a new module layout. LXC startup acceptance is deferred until User creates the environment. The suggested Rust crates are directional; the upstream implementation determines the extension strategy.
2. **Ownership:** Enforce permanent-agent ownership on sessions, tasks, threads, files, memory, events, and capability grants from their first implementation. Separate filenames alone do not provide access isolation.
3. **Restricted workers:** P03 may establish lifecycle and structured handoff using constrained tools. Broad shell/network/package/credential capabilities require the completed policy, Guard, secret broker, and sandbox gates.
4. **Execution security:** Every enabled execution entry point must route through the broker. Hard deny precedes model review. Guard errors must not become permission to execute. Single-action approvals bind agent, task, operation, target/payload, expiry, and one execution. Session approvals bind agent/task/authenticated broker identity, resource scope, fixed manifest/hash, expiry, and per-operation budgets. Authorization consumption and budget reservations are atomic; every session action still passes execution policy and Guard.
5. **External writes:** P11 is read-only until P12 passes. Shells, packages, direct HTTP, and integrations must not provide an alternate external write path. One fresh TOTP may authorize either one reviewed action or a reviewed bounded session; session actions need no additional TOTP while within scope/budgets/expiry. Codes and credentials stay outside model/chat/log context. Retrying an uncertain write requires execution reconciliation rather than blind replay, even if a gateway reports `retry_allowed:true`.
6. **Early reliability:** Add versioned migrations, persistence recovery, audit correlation, and backup/restore checks when state first appears. P24 hardens these foundations. User authentication and resource authorization must precede remote API exposure.
7. **Human-controlled lifecycle:** Temporary worker cleanup is expected; durable thread/workspace archival and maintenance changes require User's decision. Memory promotion remains selective, source-aware, and sensitivity-checked.

## Existing n8n write integration reference

User supplied the current email-management gateway on 2026-10-05. P11–P12 should adapt its request-manifest → review → TOTP-authorize → action → end contract. It grants a fixed allowlist and per-operation budgets for at most five minutes; it handles mailbox organization, not sending or deletion. Purpose text does not constrain particular folders/messages/tasks, so runtime integration must add and enforce the reviewed resource and ownership scope.

Before enabling writes, validate source/agent/task binding, secure code/token handling, concurrent budget/replay behavior, revocation, restart recovery, and uncertain-result reconciliation. The export uses workflow static data and a rolling audit history; transactional shared authorization state and durable runtime audit are acceptance concerns. This supplied example does not change P11/P12 completion status.

## P00 next-work checklist

- [x] Review existing direction, architecture, and roadmap documents.
- [x] Establish work logs and phase status tracking.
- [x] Identify the supplied fork and baseline revision: `https://github.com/unrealthind/Clawberry.git`, `61043a01a203a05d85b4e550907190005ef77a84`.
- [x] Obtain source while preserving the existing project documents.
- [x] Establish Git history and record the inspected baseline provenance/licensing; deeper upstream ancestry remains optional follow-up.
- [x] Record language/manifests and extension points: Rust 2021, `Cargo.toml`, Rust toolchain 1.92, `src/main.rs`, provider/channel/tool/memory/observer/runtime traits; principal behavioral and operational storage inventory is in `docs/baseline.md`.
- [x] Inventory the principal tools, credential handling, policy paths, memory/session schema, and execution bypass risks in `docs/baseline.md`.
- [x] Build/test upstream unchanged; capture default/full results and inherited quality failures.
- [x] Validate current core CLI and daemon with a local mock provider and temporary state.
- [x] Verify legacy TUI rendering/input/exit with an offline PTY fixture and complete scheduler/operational storage inventory.
- [x] Resolve inherited formatting/strict Clippy failures and tiny all-targets feature gating.
- [ ] Confirm production target/resources/access and non-root service startup/configuration — separate deployment acceptance, deferred by User; the Rocky development VM does not close this gate.
- [ ] Verify production deployment and restart behavior — separate deployment acceptance, deferred by User.
- [x] Select existing trait/provider/memory extension points and record implementation decisions.

## Phase completion record

For each phase, add a completion entry to `worklogs.md` with implemented scope, relevant files/commit, test commands and results, deployment or migration evidence, and remaining limitations. Update the table only after acceptance evidence exists. Do not infer completion from the presence of a planning document.


## October 19 — local product steps 1–3

Local actual Claude Fast/Normal/Deep routing is verified, the prototype defaults to live gateway data, and schema-v8 backend settings persist global preferences/profiles/budgets and separate owner identity/memory notes/skill text. Saves use revision conflicts; subsequent runtime requests use saved guidance/routing without granting new capabilities. Development uses environment credentials; production web enrollment remains planned. Thread file/decision/result completion is next, followed by unchanged Guard/n8n/TOTP/always-on deployment acceptance. This does not mark all P06–P20 complete. See [the product plan](docs/product-plan.md).


## October 19 thread and recovery delivery

Step 4 is complete locally: authenticated thread file upload/download, schema-v9 durable human decisions with idempotent retries, and structured worker results/archive downloads. Local validation: 2,445 Rust tests and 14 frontend checks passed; 43 browser checks passed with one intentional skip; five existing host sandbox Rust tests remain ignored. Both gateway modes passed complete stopped fixture-state backup/restore. The Linux service unit newline bug is fixed, and concurrent write-budget admission is tested across independent connections.

Step 5 is deferred by User; target VM, live Guard/n8n/TOTP and always-on acceptance remain open. Production credential enrollment remains unfinished. Final-head CI is required; At that historical checkpoint PR #1 was draft and main was unchanged; the later authorized consolidation superseded that restriction. See [deployment acceptance](docs/deployment-acceptance.md).
