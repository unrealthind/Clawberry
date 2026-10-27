# Working Logs

This file records development work, findings, validation, and the next actionable step. Phase status lives in [phases.md](phases.md); detailed scope lives in [ROADMAP.md](ROADMAP.md). Product constraints come from [AGENTS.md](AGENTS.md), and the target design is in [ARCHITECTURE.md](ARCHITECTURE.md).

## Logging rules

- Append an entry after each meaningful work session, using the date in America/Edmonton.
- Record what actually changed and what was verified. Planned features are not completed work.
- Include affected files, phase IDs, commands/results, unresolved issues, and the next step.
- Keep credentials, TOTP codes, and private source content out of these logs.
- Preserve previous entries. Add a correction when a previous conclusion changes.
- Mark a phase complete only when its exit criteria have supporting evidence.

## Entry template

```markdown
### YYYY-MM-DD — Session title

Phases: P00
Status: completed / in progress / blocked

Work performed:
- Changes and affected files.

Validation:
- Command or review performed, result, and relevant evidence.

Findings / decisions:
- Facts, assumptions, recommendations, and unresolved issues.

Next step:
- Concrete follow-up.
```

## Entries

### 2026-10-05 — Project review and development tracking

Phases: P00; planning across P01–P25
Status: completed documentation review; P00 remains in progress

Work performed:
- Reviewed all three existing project documents: `AGENTS.md`, `ARCHITECTURE.md`, and `ROADMAP.md`.
- Inspected the local workspace for source, build manifests, tests, deployment files, and Git state.
- Created `worklogs.md` for ongoing development records.
- Created `phases.md` with implementation status, dependencies, acceptance evidence, and a mapping between the two existing phase schemes.

Validation:
- `ls -la` showed only the three planning documents before this session.
- `rg --files` with source/manifests/test filters found no implementation files.
- `git status --short` returned `fatal: not a git repository`.
- Reviewed the new Markdown files for valid local links and consistency with the detailed roadmap's P00–P25 phases.
- No build or runtime tests could run: this workspace has no source checkout or build configuration.

Review findings:

| ID | Priority | Finding and evidence | Recommended follow-up |
| --- | --- | --- | --- |
| R01 | High | The local workspace is a planning scaffold. `AGENTS.md` describes a fork, but no upstream source or Git metadata is present. This does not establish whether a fork exists elsewhere. | Identify the intended upstream repository and revision, then establish a reproducible P00 baseline. |
| R02 | Medium | `AGENTS.md` uses phases 0–9, while `ROADMAP.md` expands the work into phases 0–25. Unqualified phase references are ambiguous. | Use detailed roadmap IDs P00–P25 in implementation tracking; retain the broad phase mapping in `phases.md`. |
| R03 | High | Workers arrive in roadmap P03, but policy, Guard, secrets, and sandboxes arrive in P06–P09. Upstream tools might already execute actions. | Establish ownership and audit foundations early; keep worker execution restricted until the complete broker/security/sandbox path passes acceptance checks. Inventory and constrain upstream tools during P00. |
| R04 | High | P11 includes write workflow calls before P12 establishes the TOTP boundary. | Develop P11 using read-only workflows; keep writes disabled until P12 verifies approval, TOTP, expiry, replay protection, and payload binding. |
| R05 | High | Separate databases are specified, but a process able to open both private databases could still bypass isolation. The sandbox mechanism and database access identities are not selected. | Define the trusted memory service boundary and OS-level access model; test direct DB, workspace, archive, and secret access denial before enabling agent tools. |
| R06 | High | Guard timeout/unavailability/malformed-result behavior and approval consumption are not specified in enough detail to implement safely. | Define fail-closed execution behavior and atomic approval use. A deep review must not override root policy or silently grant execution. |
| R07 | High | The proposed gateway API includes memory, audit, chat, and approval endpoints without an explicit authentication/authorization contract; secure sessions appear later under desktop packaging. | Add authenticated ownership checks to the initial API, including uploads and WebSocket subscriptions, before exposing the service. |
| R08 | Medium | Recovery, migrations, backups, and idempotency are mostly grouped under P24 despite earlier persistent tasks and external integrations. | Introduce migrations and restore checks with P01/P02, and execution reconciliation/idempotency with P11/P12; use P24 for hardening. |
| R09 | Medium | The architecture calls audit records append-oriented but does not define how agent/runtime identities are prevented from altering stored history. | Specify storage ownership, append interfaces, retention, and tamper detection; verify model-facing shells cannot mutate audit storage. |
| R10 | Medium | Provider/model names and an existing TOTP workflow are design targets, not locally verified integrations. No upstream URL, LXC details, or workflow contract is supplied. | Verify provider availability and workflow behavior when implementing the integrations; keep provider IDs configurable behind effort profiles. |

Assessment:
- The product documents consistently prioritize peer agent ownership, ephemeral workers, brokered secrets, human control, and a chat-first UI.
- Implementation readiness is limited by the absent upstream baseline and unresolved execution contracts. No runtime behavior or security enforcement has been demonstrated locally.
- Existing design documents were preserved. The safeguards in `phases.md` are implementation recommendations derived from their requirements, not claims that features already exist.

Next step:
- Identify TinyClaw's intended upstream URL/revision and target LXC environment, obtain the source without overwriting these documents, and record unchanged build/test/startup evidence for P00.

### 2026-10-05 — Supplied GitHub repository review

Phases: P00; security implications for P06–P12
Status: completed source review; build/deployment baseline remains open

Source:
- Repository supplied by User: https://github.com/unrealthind/Clawberry.git
- Branch: `main`; inspected commit: `61043a01a203a05d85b4e550907190005ef77a84`.
- Commit subject: `feat: add file_patch, search_files, web_fetch tools + CLI/Anthropic streaming`.
- Source cloned into `/private/tmp/raaviclaw-review-20261005` for review. No source import, push, or runtime configuration changes were made.

Correction to initial scope:
- R01 describes the local workspace only. A real TinyClaw implementation exists in the supplied remote repository.
- The remote `AGENTS.md` is an inherited ZeroClaw coding guide. The local `AGENTS.md` contains the new RaaviClaw product requirements. Source integration must preserve the new product requirements and reconcile useful inherited engineering guidance.

Existing implementation and reuse candidates:
- `Cargo.toml`: Rust 2021 package `tinyclaw`, MIT license, tiny/standard/full feature tiers; `rust-toolchain.toml` specifies 1.92.
- `src/agent/loop_.rs`: agent orchestration and tool dispatch; `src/session.rs`: session persistence.
- `src/providers/traits.rs`, `src/channels/traits.rs`, `src/tools/traits.rs`, `src/memory/traits.rs`, `src/observability/traits.rs`, `src/runtime/traits.rs`: existing modular extension points.
- `src/runtime/mod.rs`: native and Docker runtime factories. Actual containment has not been tested.
- `src/security/policy.rs`, `src/security/secrets.rs`, `src/security/pairing.rs`: deterministic command/path policies, encrypted secret storage, and gateway pairing foundations.
- `src/gateway/mod.rs`: pairing and bearer authentication already exist. R07 applies to ownership authorization and new API design, not a claim that the existing gateway lacks all authentication.
- `src/memory/sqlite.rs`: existing `memories` and `embedding_cache` tables. `MemoryEntry` includes category/session/retrieval score, but lacks the planned owner, confidence, provenance, sensitivity, and verification fields. Retrieval score must not substitute for factual confidence.
- `src/daemon/mod.rs`: heartbeat worker invokes the agent on configured tasks. `src/heartbeat/engine.rs` reads task lines from `HEARTBEAT.md`; this needs per-agent scheduling, evidence/recommendation flow, and execution gates.
- `src/providers/traits.rs`: usage tracking exists, but estimated cost uses fixed input/output rates. This is not the planned durable, per-agent/task/provider cost ledger.

Source findings:

| ID | Priority | Evidence | Required follow-up |
| --- | --- | --- | --- |
| R11 | High | `src/tools/shell.rs:49` exposes `approved` in the model-facing schema; execution reads it at line 64 and passes it to policy. `src/security/policy.rs:292` accepts that boolean for risk approval. `src/agent/loop_.rs:208` dispatches parsed tool arguments directly. | Replace model-supplied approval with authenticated, broker-owned approval lookup and atomic scope consumption. Preserve independent hard denials. Source tracing demonstrates the trust problem; no exploit was executed. |
| R12 | High | `src/tools/composio.rs:63` executes named actions directly through HTTP POST; `src/tools/mod.rs` registers the tool when a Composio key is configured. This path has no n8n/TOTP check in the inspected tool implementation. | Keep unsupported external action paths disabled for the new platform. Route supported writes through n8n/TOTP and explicitly classify any retained read operations. |
| R13 | Medium | Remote README/code still use TinyClaw/ZeroClaw names and legacy repository references; remote `AGENTS.md` differs from the local product instructions. | Reconcile documentation when source is imported; retain upstream attribution and avoid treating inherited instructions as proof of new product implementation. |

Validation:
- `git ls-remote` confirmed HEAD and `main` at the recorded commit.
- A shallow clone succeeded; inspected tracked source, manifests, inherited guide, tool registry, policy, memory contracts, gateway authentication, and heartbeat implementation.
- Case-insensitive search of `src/` found no matches for `janet`, `harley`, `totp`, `n8n`, or `llama.guard`. This is a source-search result, not exhaustive proof of absence under all possible names.
- Cargo is not in PATH (`command -v cargo` produced no path). Builds, tests, and live LXC checks were not run. README performance/test counts are unverified claims.
- Updated `phases.md` with the remote baseline and existing reuse candidates; implementation phases remain uncompleted.

Next step:
- Integrate the source into the working project while preserving local planning documents, establish a Rust 1.92 build environment, and complete P00 baseline checks. Address R11/R12 before granting operational agent capabilities under the new design.

### 2026-10-05 — Source integration, baseline validation, and Phase 1 core

Phases: P00, P01
Status: P00 in progress; P01 conversation-only local core complete

Scope decision from User:
- Build the core program first. User will create the LXC later, after which deployment work will proceed together. No LXC access or deployment was needed for this session.

Git delivery:
- Imported the supplied source and Git metadata into this workspace, preserving local planning files.
- Retained inherited instructions as `UPSTREAM_AGENTS.md` and kept the new product requirements in `AGENTS.md`.
- Committed and pushed the planning files to `main` as `6697f6b` (`docs: add RaaviClaw architecture, phases, and work logs`).
- Prepared the core implementation on `feat/multi-agent-foundation` to keep the planning baseline independently reviewable.

P00 work:
- Installed Rust 1.92, rustfmt, and Clippy into `/private/tmp/raaviclaw-cargo` and `/private/tmp/raaviclaw-rustup`; no global shell configuration changes.
- Tested unchanged baseline `61043a01a203a05d85b4e550907190005ef77a84` in the separate review checkout.
- Added `docs/baseline.md` with source/license, runtime/module/storage inventory, extension choices, baseline results, and inherited security/quality gaps.
- Adopted existing traits/backends instead of introducing a new crate hierarchy or dependencies.

P01 work:
- Added compiled Janet/Harley identities/configuration under `agents/`.
- Added `src/assistants/mod.rs`: persistent registry, owned main sessions, runtime-authored message envelopes, versioned SQLite system schema, separate workspace/memory paths, and correlated chat audit events.
- Added `tinyclaw agents`, `tinyclaw chat janet`, and `tinyclaw chat harley`; added `--config-dir` for isolated runtime development without changing HOME.
- Added authenticated `GET /api/agents` and `POST /api/chat/:agent` to the existing gateway, enabling both identities in one daemon.
- Enforced history ownership and bounded context; atomic successful-turn persistence; per-agent serialization and cross-host revision conflict detection; provider timeouts and explicit failure audit.
- Added isolation/security integration tests and `scripts/test_assistant_smoke.py` with localhost-only fixtures.
- Added `docs/permanent-agents.md` for usage, contracts, acceptance, limitations, migration, and rollback; linked it from README.

Execution boundary:
- The new host never registers tools, dispatches model tool text, enables autonomous heartbeats, inherits tool credentials, or sends cross-agent messages.
- Empty tool/credential grants are part of each permanent identity; startup rejects attempts to enable these unsupported capabilities.
- Provider credentials remain infrastructure inputs; fixture smoke tests confirm they are absent from model contexts.
- R11/R12 remain inherited legacy execution issues. Their tools are not connected to the new host; actual broker/root-policy/Guard/TOTP remediation remains the corresponding security phases.
- Isolation is enforced by trusted host context/storage routing with private directories. OS-level sandbox enforcement and append-protected audit storage remain future work.

Validation:

| Check | Result |
| --- | --- |
| Unchanged default/full baseline `cargo test --locked --all-targets` | Passed: 1,119 library + 1,120 binary + 27 integration = 2,266 executions |
| Current default/full `cargo test --locked --all-targets` after package artifact cleanup | Passed: 1,122 library + 1,123 binary + 36 integration = 2,281 executions |
| Current tiny-core `cargo test --locked --no-default-features --features tiny --lib --bin tinyclaw --test assistant_isolation` | Passed: 773 library + 774 binary + 9 isolation = 1,556 executions |
| `cargo build --locked` | Passed with inherited compiler warnings |
| Local mock-provider CLI/daemon smoke script | Passed: both identities, authenticated routes, caller spoof rejection, persistent selected-owner context after reopen |
| Rustfmt check on all modified Rust files; `git diff --check` | Passed |
| Unchanged `cargo fmt --all -- --check` | Failed on inherited formatting differences |
| Strict Clippy, unchanged and current | Failed on inherited diagnostics; current library-test count remains 137, with no diagnostics in the new assistant module/tests or new gateway handlers |
| Tiny all-targets integration command, unchanged and current | Inherited failure: WhatsApp security integration imports feature-gated gateway; targeted tiny-core tests pass |
| Real providers, TUI interaction, LXC deployment | Not verified; no real provider credentials used; LXC explicitly deferred |

Test execution counts include duplicated unit tests from library and binary compilation; they are not unique-test counts. Baseline and implementation checkouts initially shared Cargo target artifacts; stale package outputs were detected, cleaned, and the final core suites rerun. Use independent target directories for separate checkouts.

Next work:
- Continue P00 quality cleanup and remaining operational/TUI inventory, then P02 tasks and Focus Threads.
- Keep execution capabilities disabled until their full security and sandbox acceptance gates pass.

### 2026-10-05 — Complete the local core through P05

Phases: P00–P05 (detailed `ROADMAP.md` numbering)
Status: complete for the authorized local core; LXC deployment deferred

User requested completion through P05. P05 means the model router in the established tracker; n8n/TOTP remain P11–P12. Core coding continues on `feat/multi-agent-foundation`.

Work performed:
- P00: normalized inherited formatting, applied mechanical Clippy fixes, retained narrowly commented API/Serde/legacy scoring/style exceptions, made telemetry/TUI casts saturate safely, serialized process-global environment override tests after catching an inherited race, and gated WhatsApp integration tests correctly. Completed cron/heartbeat/daemon/service storage inventory and offline legacy TUI acceptance.
- P02: added owned durable tasks, events, dependencies/cycle refusal, terminal-state rules, priority/due date, Focus Threads with separate history/revisions, compact main-chat relevance hooks, scoped attachments/checksums, and inspection operations. No automatic thread archival.
- P03: added temporary asynchronous model workers with selected context, empty tool/credential grants, owned workspace, profile and budget, bounded timeout, cancellation, status/wait/retry, structured result validation, persisted delegation/result envelopes, archival, and active-entry/empty-workspace cleanup. Broad execution capabilities are refused.
- P04: added atomic archive publication, transcripts/raw results/failure evidence, sources/costs/artifacts/checksums, pending candidates, selective promotion/rejection, confidence/provenance, and sensitive-category exclusions. A promoted fact and candidate state commit together into system/owner-private stores. Logs are never bulk-promoted.
- P05: added administrator-configurable fast/normal/deep profiles, current Anthropic presets, new-config Sonnet default, bounded provider output, retained non-streaming history, checked integer microUSD accounting, atomic cross-host reservations, agent/deep/day/task/worker budgets, explicit deep escalation and opted-in worker retry escalation. Missing usage and interrupted requests retain conservative charges; refusals do not downgrade reasoning.
- Extended CLI commands, authenticated owner-operation/chat routes, and owner-scoped operation request/result logs. Added schema-version-5 migrations preserving version-1 main histories, expired-lease/pending-archive recovery, and explicit refusal of newer schemas.
- Added `docs/core-foundation.md`, profile example, work/migration/budget tests, and comprehensive local fixtures. Updated README, baseline/permanent-agent guides, and phase tracker.

Validation:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed |
| Default/full `cargo test --locked --all-targets` | Passed: 2,292 executions |
| Tiny `cargo test --locked --no-default-features --features tiny --all-targets` | Passed: 1,586 executions |
| Standard `cargo test --locked --no-default-features --features standard --all-targets` | Passed: 1,586 executions |
| `cargo build --locked` | Passed |
| `scripts/test_assistant_smoke.py` | Passed: paired one-daemon/CLI routing, owned threads/tasks/files, restricted workers, archives, candidate review, profile/token limits, explicit escalation, cancellation/timeout/retry, uncertain usage, budget preflight refusal, restart and ephemeral cleanup |
| `scripts/test_tui_smoke.py` | Passed: offline legacy TUI rendering, input editing, clean exit |
| Migration/recovery tests | Passed: version 1 → 5 preserves main messages; expired workers archive as interrupted; pending cancellation remains cancelled; cleanup/recovery is repeatable |
| Concurrent budget tests | Passed: independent hosts reserve atomically; rejected deep/cross-owner task calls create no usage |

Checks used the pinned Rust 1.92 toolchain and locked cached dependencies (`--offline`). Counts include duplicated library/binary unit-test executions. Provider behavior is verified with local fixtures, not live API credentials. During the first TUI fixture, inherited skill loading created `~/open-skills`; the test-created checkout was verified and removed, and the fixture now disables community downloads/sync explicitly. The new host never loads those skills.

Boundaries and remaining work:
- P03 workers are temporary model tasks, with no shell/direct network/file/database/secret tools. They can analyze selected supplied material and return text artifacts; autonomous web/media work awaits the guarded execution phases.
- New-host external execution/write paths, cross-agent communication, and heartbeats remain unavailable. Legacy tools remain separate and require the P06–P12 broker/security work before integration.
- P04 implements selective reviewed facts; general semantic-memory retrieval/edit/delete/verification/shared-memory rules remain P14.
- Cost records use configured prices; uncertain billing remains charged, with no silent refund/downgrade. Provider invoices, cached discounts, and live provider availability are not certified by fixtures.
- Gateway retains its 64 KiB request-body and 30-second request timeout. CLI attachment limits are larger; long workers should be inspected by status polling.
- Back up the complete stopped state tree before upgrading. The P01 binary refuses schema 5; rollback requires the matching version-1 backup.
- LXC OS/resources/service/deployment verification remains explicitly deferred until User creates it.

Next step: P06 root-of-trust and deterministic policy foundation, preserving these ownership and execution boundaries.

### 2026-10-05 — Begin P06–P10 guarded-execution foundation

Phases: P06–P10
Status: in progress; no permanent-agent execution capability enabled

Work performed:
- Added a separate `ExecutionBroker` with deterministic root-of-trust denials, fail-closed Guard decisions, and exact, atomic single-action approval consumption. Approval scope binds agent, task, capability, operation, resource, and payload hash.
- Added a Guard interface and explicit `NeedsDeepReview` outcome. A blocked or unavailable Guard never reaches approval/execution; review may only formulate a safer request.
- Added the named `SecretBroker`: explicit agent/capability grants, encrypted persisted values, and a trusted injection callback. Models can receive credential names only.
- Hardened the Docker execution adapter with all capabilities dropped, `no-new-privileges`, a PID limit, and a bounded `noexec,nosuid` scratch mount. The existing default network-off, read-only root, CPU/memory-limit, and workspace-root checks remain in force.
- Added P10 media admission metadata: owned task binding, safe filename/type/size limits, provenance, and SHA-256 checksum. No decoder or execution path is enabled.
- Added [execution-foundation.md](docs/execution-foundation.md) with the required broker ordering and deliberate limits. P06–P10 are now recorded as in progress in `phases.md`.

Validation:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --locked --lib` | Passed: 1,131 tests |
| `git diff --check` | Passed |
| `cargo clippy --locked --lib -- -D warnings` | Blocked by 33 inherited diagnostics when Homebrew Rust 1.99 was used instead of the repository-pinned Rust 1.92; no reported diagnostic named the new P06–P10 modules |

Remaining gates: configure a host-owned non-writable root policy; connect the live Llama Guard provider and controlled deep-review flow; wire audit records to the persistent host audit table; design package-operation logging and LXC escape tests; then implement sandboxed file/media processing. n8n/TOTP external writes remain P11–P12 and are still disabled.

### 2026-10-06 — P06–P20 components, workflow examples and app foundation

Status: milestone incomplete; local changes not committed or pushed.

Implemented root-owned default-deny policy loading, audited atomic single-use approvals/replay protection, a fail-closed Guard HTTP client and non-executing review schema. Hardened encrypted secret storage, atomic manifest updates, rotation, scoped injection, nested output redaction and identifier-only usage audit. Added reviewed read-only n8n contracts and a durable scoped write-session ledger with budgets, expiry/revocation and uncertain-outcome reconciliation. These are components, not live TOTP authorization or enabled external writes.

Added audited private-memory CRUD/relevance retrieval and exact, expiring, human-reviewed peer exchanges without private DB sharing. Added authenticated bounded file transfer, a React chat-first app with Focus Threads, memory management, activity and peer review, and closed card-schema tests. Model workers are bounded to two globally and one per owner. User selected one four-core, 6 GB Proxmox VM; remote inference remains the RAM-efficient direction.

Reviewed supplied IMAP/SMTP/read-only exports as integration examples. No export, raw credential or email content was copied into the repository, and no live n8n call was made. Read-only and TOTP write paths remain distinct. See [phase-by-phase acceptance ledger](docs/p06-p20-progress.md).

Validation: `cargo test --locked --tests --quiet` passed 1,154 library, 1,151 binary and 41 integration tests before two additional example-config tests. The tiny suite passed 803 library, 800 binary and 33 integration tests. App production build and both card tests passed. Formatting and diff-whitespace checks passed. Clippy still reports 33 inherited diagnostics under Homebrew Rust 1.99; strict Clippy acceptance is not claimed.

No browser/mobile visual acceptance: no connected browser was available and native Safari computer-use permission was denied. No VM, live Guard, sandbox escape or deployed workflow acceptance was performed. P13 scheduling, guarded shell/media execution, live TOTP wiring, shared-memory promotion, streaming and production asset serving remain unfinished. Permanent-agent execution, write actions, model peer capabilities and autonomous heartbeats remain disabled.

### 2026-10-06 — RaaviClaw brand identity

Created an original monochrome split-stem R mark, horizontal SVG logo, light mark, SVG favicon, unmasked app-icon master and PNG exports from 16px through 1024px plus an Apple touch icon. Replaced product display names in app/login, CLI/TUI, onboarding, gateway, daemon, channel and service output. Removed inherited oversized onboarding ASCII branding. Kept executable/configuration/service identifiers stable and preserved upstream attribution and licenses.

The Apple design-review skill informed small-size silhouette, centered unmasked native artwork, accessible text beside decorative marks and unobtrusive placement. Repo-native vectors were used instead of image generation; the checked-in macOS export script adds no runtime dependency. Brand use and compatibility scope are documented in `docs/branding.md`.

Validation: inspected 512px and 16px raster exports; production app build and all five frontend tests passed (shared geometry, PNG dimensions, accessible identity and card validation). Full Rust suite passed 1,156 library, 1,152 binary and 41 integration tests before the final onboarding-banner/help-copy-only adjustments. No browser/device visual acceptance or trademark clearance is claimed.


### 2026-10-06 — P06–P12 connected runtime and Debian VM acceptance

Status: implemented local integration; milestone remains incomplete; changes uncommitted.

Connected permanent-agent execution to host-owned policy, live Guard admission, advisory deep review, exact human exception approvals, encrypted scoped credentials and audit. Added rootless Podman shell/package/media execution, bounded resources/output, timeout cleanup and request staging cleanup. Legacy CLI/TUI model-tool registries are disabled and the model-provided shell approval flag removed. Execution capabilities remain administrator opt-in.

Connected reviewed n8n reads and prepared write manifests to a dedicated human TOTP endpoint, short-lived encrypted gateway sessions, atomic write budgets, uncertain-outcome handling, explicit reconciliation and session revocation. Added exact-manifest review and authorization controls to the app. Skills use named core integrations; webhook transport/security belongs to the runtime. No live external write was performed.

The current Debian 13 Proxmox VM now has working rootless Podman and a root-owned default-deny policy. The reviewed sandbox image is pinned in deploy/execution.vm.toml. Live execution configuration, Hugging Face credentials and deployed n8n protocol contracts are still absent.

Validation: full locked/offline Rust suite passed 1,171 library, 1,172 binary and 42 integration tests; four optional real Podman tests passed separately (root policy, isolation/resource/output/timeout controls, image preview and short-video filmstrip). Tiny suite passed 817 library, 818 binary and 34 integration tests before the final legacy-registry changes. Strict pinned-toolchain Clippy, formatting and whitespace checks passed before the final staging-cleanup adjustment; final checks follow below. App production build and seven frontend tests passed. Assistant CLI/daemon smoke and TUI smoke passed.

Remaining acceptance and implementation: live Guard/deep-review and deployed n8n/TOTP round trip; persistent task-installed packages; scoped worker execution grants; restart orphan-container reconciliation; larger media/attachment ingestion; browser/mobile review. See docs/p06-p12-runtime.md and docs/p06-p20-progress.md. P06–P12 are not marked complete.

Final staging-cleanup checks: full Rust suite, strict all-target Clippy, formatting and diff-whitespace checks passed.

### 2026-10-06 — Existing read-only email compatibility

P11 remains in progress. Prepared Janet endpoint contract for the supplied read-only
workflow, matching Bearer authentication without changing n8n response shape.
Added opt-in email_readonly_v2 response admission with operation/mailbox/UID and
shape checks, explicit error rejection and reviewed-field projection. Existing
contracts continue requiring ok:true. Partial mailbox errors remain visible.
Corrected setup instructions to preserve the existing workflow for other agents.

Validation: cargo test --locked --lib n8n::tests --quiet passed all five tests;
cargo fmt completed and git diff --check passed. No live HF/n8n request or remote
deployment performed. Next: secure email credential registration and live Guard/read
acceptance; verify actual mailbox names before enabling the prepared contract.

### 2026-10-06 — VM read-only deployment preparation

Confirmed current workspace on Debian VM. Updated cargo build --locked --offline
passed. Prepared deploy/execution.email-readonly.vm.toml, enabling only Janet's
n8n.read capability and reviewed contract; writes remain empty. No host policy
was modified. Root policy remains default-deny and execution.toml absent.
A CLI task-create attempt failed with read-only filesystem before acceptance
could proceed: assistant access to default state lies outside writable roots.
Manual host-administrator configuration and terminal acceptance remain pending.

### 2026-10-06 — Anthropic sampling compatibility fix

User live review request returned HTTP 400: temperature deprecated for the
selected model. Updated all Anthropic request paths to omit temperature for
modern/unknown models, preserving known legacy generation sampling settings.
Verified against Anthropic parameter deprecation documentation. No Guard or
approval behavior changed and no live email was read by this fix.

Validation: 17 Anthropic provider tests passed, locked offline CLI build passed,
strict library Clippy passed, formatting and diff-whitespace checks passed.
Next: user reruns live read request to obtain deep-review result; Guard block
remains unresolved and may not be silently overridden.

### 2026-10-06 — Anthropic mixed-content response compatibility

User live deep review failed decoding response body. Inspection found required
text on every content block, incompatible with thinking/non-text blocks. Added
tagged content parsing, discarding non-text payloads and combining text answers;
no-text and malformed text responses remain errors. Likely cause addressed; live
confirmation pending. Supplied original email skill inspected as reference only,
not imported; its automatic mark-read/write behavior is not enabled.

Validation: 18 provider tests, locked offline CLI rebuild, strict library Clippy,
formatting and whitespace checks passed. No live model/email request performed
by the assistant. Next: retry live review and inspect its structured result.

### 2026-10-06 — Strict deep-review output compatibility

Following live JSON parsing failure, replaced shorthand deep-review instructions
with exact JSON schemas. Added parsing of a single complete JSON object with
optional JSON fence; prose, unknown decisions/fields and partial JSON remain
rejected. Invalid review is audited without retaining free-form model output,
and reports execution remains blocked. No silent approval or dispatch added.

Validation: three Guard/review tests passed, locked offline CLI rebuild and strict
library Clippy passed; formatting/whitespace checks passed. Live retry remains
necessary; actual rejected model output was not available for inspection.

### 2026-10-06 — User-confirmed live read-only email acceptance

User ran the prepared exact exception request in the VM terminal and supplied
a successful runtime result: Janet, email_work_readonly, recent INBOX max=1;
count=1, read_only=true, mailbox_errors empty. No email content or identifiers
are retained in this log. This establishes a live HF Guard block/deep review/
scoped exception/read workflow round trip. The approval/run sequence was user
executed; no writes were performed. Gateway audit/execution IDs were null.

P07/P11 remain in progress: diagnose Guard false-positive/context formatting;
complete negative/replay/restart live checks and durable gateway correlation.
Write workflows remain disabled and shared TOTP replay acceptance is pending.

### 2026-10-06 — Replay, restart and correlation checks

Added restart test showing exact exception review/grant survives DB reopening,
consumption remains spent, review cannot be reused, and one admitted audit row
retains approval correlation. Found SAFE autonomous-read admission did not reserve
request IDs; fixed with atomic durable admission reservation. Regression proves
a Guard verdict changing from BLOCK to SAFE cannot replay a consumed exception,
and fresh autonomous reads cannot reuse their identity. No privileges expanded.

Validation: 13 execution tests passed; all 1,177 library tests passed; locked
offline CLI build, strict library Clippy, formatting/whitespace checks passed.
Live service restart not tested (runtime state inaccessible in tool sandbox).
Gateway audit/execution IDs still absent. Guard category is discarded by current
adapter; actual live block cause remains unknown. No policy/Guard bypass or live
email call performed during these checks.

### 2026-10-06 — Live Guard diagnosis and gateway restart acceptance

Approved tool escalation now permits scoped runtime acceptance against VM state.
Live Guard-only diagnostic returned BLOCK/S14 (Code Interpreter Abuse) for the
existing bounded email-read request; no email call was dispatched. Actual input
classification cause remains unresolved; category alone does not justify bypass.
Added validated category-only Guard reports and correlated runtime assessment
audit; raw provider responses/credentials are not retained.

Read-only live DB checks confirm one admitted action, correlated approval with
zero remaining uses, and runtime review/approval/start/completion events. Existing
user/system tinyclaw services are inactive; user unit is not installed. Ran two
temporary localhost gateway starts/stops using existing state: health passed,
consumed approval preserved, processes cleaned up. This verifies gateway process
restart, not full daemon/systemd service deployment.

Added optional X-N8n-Execution-ID response-header ingestion with bounded values
and conflict rejection. Prepared contract/docs updated; actual workflow/header
and host contract installation are pending manual administrator action. Existing
workflow body is preserved. No gateway ID is fabricated; original live response
had no gateway audit/execution IDs. No raw email content copied to worklog.

Validation: all 1,179 library tests passed, six n8n tests/four Guard tests passed,
locked offline CLI/example builds and strict all-target Clippy passed; formatting
and whitespace checks passed. Guard diagnostics and localhost restart acceptance
passed as above. Next: manual n8n header configuration and protected contract
refresh, live correlation check, Guard format/control investigation and service
installation acceptance. External writes and permanent heartbeats remain disabled.

### 2026-10-06 — User-confirmed live n8n execution correlation

User executed a freshly reviewed one-message recent INBOX read through the
exact human Guard exception path. Successful result included n8n_execution_id
3356 and result.execution_id 3356, count=1 and empty mailbox_errors. No email
content retained here. This confirms deployed response header, reviewed contract
projection and runtime execution correlation end to end. gateway_audit_id remains
null; no separate gateway audit identifier is claimed.

Guard still blocks the read with S14; autonomous SAFE-path acceptance remains
open. Full systemd service is not installed; temporary gateway restart passed
earlier. P07/P11 remain in progress and writes remain disabled.

### 2026-10-06 — Documentation and branch delivery preparation

Updated README, phases, P06–P20 ledger, runtime and read-only setup documentation
to distinguish live bounded exception reads/correlation and temporary gateway
restart from unresolved S14 classification/full service/write acceptance.
Generalized host diagnostic task/request IDs and excluded short-lived local
approval payloads from Git. All pending connected-runtime changes are included
in the requested feature-branch delivery; no main-branch merge is requested.

Final validation: cargo test --locked --offline --quiet passed 1,179 library,
1,180 binary and 43 integration tests (four optional Podman checks ignored in
this run; earlier real VM acceptance recorded separately). Strict all-target
Clippy, fmt and whitespace checks passed. All seven frontend tests and production
build passed using checksum-verified temporary official Node 22.16.0 under /tmp,
without installing host packages. Credential-format scan found no matches in
versionable text files. Commit/push requested by User.
### 2026-10-16 — Read-only workflow/app checkpoint toward P20

Status: partial implementation; paused at User's request. User deferred deployment until coding is ready and then authorized committing/pushing this checkpoint. P06–P20 are not marked complete. Remaining work is recorded in [the checkpoint guide](docs/coding-checkpoint-2026-10-16.md).

Changes:
- Added bounded newest-first human history pages, separate from the 50-message model context, and complete private-memory pagination/server-side filters. Relevant fact retrieval can find facts older than the first 200 records.
- Added independently configurable read-only watch schedules, explicit UTC quiet hours, atomic cross-host claims, conservative daily run/budget limits, owned stale/due/failed-read evidence, restricted investigation workers and durable recommendation/failure notifications under Activity. Schedules default off; no tools/credentials or external write capability are granted.
- Added exact human-reviewed promotion of ordinary/current facts into separate `data/shared.db`, relevant shared context for both assistants, and audited shared-copy revocation. Binding changes, wrong-owner access, stale/expired/sensitive promotion and duplicate copies are rejected.
- Initially added a human named-read adapter locally. During push, discovered and integrated newer remote `018107d` with the more complete guarded `n8n.read` service; discarded the redundant local adapter/configuration. Preserved the remote's capability checks, response projection, Guard audit, deep review, dedicated TOTP service/UI and sandbox/media code.
- Added authenticated SSE chat, provider-delta forwarding with single-result fallback, suppression of internal-action payloads, Stop/disconnect semantics and cancelled/uncertain inference accounting. Final conversation persistence remains atomic; completed internal operations remain auditable. Hardened Anthropic stream UTF-8 handling, completion/error checks, response bounds and final usage accounting.
- Added optional same-origin production app serving with containment/MIME/size checks and browser security headers. API misses do not serve app HTML.
- Migrated system schema 6 → 7 for watches/notifications; existing ownership/history migrations remain tested. Added checkpoint/migration/configuration notes and updated app/tracker documentation.

Pre-integration verification:
- Default/full `cargo test --offline --locked --all-targets --quiet`: passed 1,165 library + 1,165 binary + 41 integration executions (2,371 total, including duplicate library/binary unit tests).
- Tiny `cargo test --offline --locked --no-default-features --features tiny --all-targets --quiet`: passed 813 library + 813 binary + 33 integration executions.
- Final targeted assistant tests after cleanup: 16 passed.
- App production build, all seven frontend tests (including split UTF-8/SSE boundaries and incomplete/error stream handling), and Prettier check passed.
- Extended local CLI/daemon smoke fixture passed, including authenticated SSE, internal-operation suppression and persisted final history. Localhost sockets required sandbox escalation; no live credential/provider/workflow was used.
- Rustfmt and diff-whitespace checks passed.
- Strict Clippy under Homebrew Rust 1.99 still fails on inherited diagnostics (33 library / 74 library-test diagnostics); no diagnostics remain in the new assistant/gateway modules. Clean strict-Clippy acceptance is not claimed.

No deployment, browser/device visual acceptance, live Guard/n8n call, TOTP authorization or external write was performed in this batch. Remote work already includes guarded execution/media, deep review and dedicated TOTP integration; its recorded acceptance evidence is preserved above. Remaining integration/acceptance gaps, richer cards/threads and full shared-memory management remain open. No unrelated local files are included.

Combined-branch verification after integrating remote `018107d`:
- Preserved all remote guarded execution, deep-review, TOTP, media, credential and read-workflow changes; retained TOTP-input rejection on the new streaming host path. Removed the redundant local read adapter/configuration.
- Full Rust suite passed: 1,188 library + 1,189 binary + 42 integration executions. Four deployment-specific sandbox tests remain explicitly ignored. Fixed the remote task-mount test to compare canonical temporary paths on macOS while retaining its ownership/symlink checks.
- App production build and all nine frontend tests passed. Rustfmt and whitespace checks passed.
- Strict Clippy under Rust 1.99 still reports 33 inherited library / 76 library-test diagnostics; none point to the changed assistant/gateway/Anthropic modules.
- Final combined-branch localhost CLI/daemon/SSE smoke fixture passed after the production binary rebuild.


### 2026-10-18 — UI workspace consolidation and draft delivery preparation

Phases: P02, P16–P20; mobile layout work toward P23
Status: verified implementation and documentation prepared for existing draft PR; main merge withheld

Work performed:
- Integrated task management, calendar deadlines, thread tasks/files/decisions/sample worker handoffs, and a separate opt-in paired gateway view for live chat/internal tasks/read-only activity.
- Added owner-validated atomic audited task.edit with terminal-state protection and runtime regression coverage.
- Added global app settings once in the User footer; consolidated agent identity/memory files, skills, credential previews and boundary descriptions under the controls beside Janet/Harley. Removed duplicate Settings and standalone Skills/Vault workspace entries and the second personality form.
- Added closed nonsecret local model-profile/budget drafts with explicit save/discard, storage-failure handling and malformed-data preservation. Provider-key enrollment remains disabled; no live configuration activation is claimed.
- Reviewed repository Markdown and updated affected README, architecture/UI/roadmap requirements, contribution/security guidance, core operation docs, phase/acceptance trackers, and checkpoint cross-references. Historical logs, inherited upstream material, asset prompts and unchanged deployment procedures are preserved. Current scope is in docs/ui-workspace.md.
- Diagnosed the old security CI failure: cargo-audit 0.22.2 does not accept --locked. Pinned that tool release and selected the committed Cargo.lock with --file, with no advisory suppression.

Validation and findings:
- git fetch origin succeeded after User switched to manual command approval. Before that, automatic approval-review capacity failures prevented elevated commands; these were availability failures, not unsafe-action findings.
- GitHub main remains at 6697f6b; feat/ui-workspace at b04559f before follow-up, with 85 commits ahead and no divergence. Existing draft PR #1 targets main from the included foundation branch.
- Frontend build and all 13 unit/API tests pass. Full locked/offline Rust all-target tests pass: 2,434 executions and five explicitly host-dependent sandbox tests ignored; tiny and standard suites also pass. Local Rust is Homebrew 1.99; its new Clippy lints do not substitute for required pinned Rust 1.92 CI.
- Browser tests identified model/budget accessible names and duplicate hidden-editor label matches; fixes preserve stable control names and scope assertions to visible dialogs. Final local browser result: 37 passed, one intentional desktop skip. Locked/offline release build, frontend/Rust formatting, whitespace and deployment shell syntax checks pass. Required Rust 1.92/security/deployment CI must pass at the pushed PR head; local ShellCheck is unavailable.

Decision / next step:
- User explicitly chose to keep the consolidated PR in draft until every deployment gate passes. Prepare/push commits and update that draft, preserving all contributors’ history. Do not merge main or enable unfinished security/write/deployment capabilities. Final-head CI and the outstanding live VM/Guard/n8n/TOTP acceptance remain gates.


Draft handoff evidence: UI/runtime/test commit 6e2f364 and CI compatibility commit 7659a7d preserve the included foundation history. npm audit reports zero vulnerabilities. The final UI/browser checks pass after fixing accessible names and scoping hidden-editor assertions; no tests were skipped to hide failures. Documentation is committed separately. The remote branches are to be advanced only by normal fast-forward push to the existing draft PR; main is not a push target. Final-head CI remains pending and all deployment gates remain open as listed in docs/merge-readiness.md.


Delivery update: commits 6e2f364 (workspace), 7659a7d (security CI) and 03c28b6 (documentation) were pushed atomically to both feat/ui-workspace and feat/multi-agent-foundation. Draft PR #1 was updated and verified with isDraft=true; its base/main remains 6697f6b. Final-head CI started; frontend dependency audit passed and other jobs were pending at inspection. A final test-only portability follow-up moves browser screenshots from Mac-specific paths to Playwright per-test output directories. Main remains unchanged, and no merge or deployment is authorized until the recorded gates pass.


Security follow-up: final-head CI at 202ac39 passed the pinned Rust 1.92 required gate, frontend, browsers, deployment-static/image, workflow sanity, npm audit and cargo-deny. RustSec now executes correctly and reported six quinn-proto 0.11.13 advisories: RUSTSEC-2026-0037, 0185, 0338, 0339, 0340 and 0342. Their highest patched minimum is 0.11.18, verified against RustSec. Ran a targeted `cargo update -p quinn-proto --precise 0.11.18`; Cargo also added only its required new random/CPU-feature transitive crates and adjusted references. No advisory ignore or security-policy change was added. Validation and patched-head CI remain required; draft/main/deployment decisions are unchanged.


Patched-lockfile local validation passed: full/tiny/standard locked offline suites and release build, with the same five host-dependent test ignores; formatting and whitespace are clean. The complete package-version diff removes only quinn-proto 0.11.13, adds 0.11.18 and its five required new dependency versions, and preserves all other locked versions. Final RustSec/CI rerun remains the gate for the patched commit; main is not advanced.


### 2026-10-18 — Prototype live workspace integration

Fetched origin and confirmed `feat/ui-workspace` had no incoming/outgoing commits before editing. Replaced the stripped-down connected header with the established workspace sidebar, agent portraits and mobile navigation. Connected Focus Threads now create server-owned records and use server IDs for history/task/file context. Thread context groups persisted summary, tasks, file metadata and matching worker statuses. Chat can load earlier cursor pages and request the runtime's Fast/Normal/Deep profiles. Enter sends, Shift+Enter inserts a newline; uncertain chat/thread submissions require refreshing and inspecting persisted state before retrying. Activity/file metadata load independently so partial outages leave core chat/tasks usable.

The local CLI/daemon smoke passed against a temporary database and fixture model, including restart, authenticated SSE, tasks/threads/files and owner isolation. This does not establish live Claude, VM, Guard or external-write acceptance. Validation: frontend production build and all 13 unit/API tests pass; the full desktop/phone browser suite passed 39 checks with one intentional skip. After the final partial-outage/profile-routing changes, all six affected live-workspace browser checks passed again. Formatting and whitespace checks pass. Browser fixtures exercise profile routing, thread IDs, pagination, context, partial failures and phone navigation. Shared configuration drafts, per-agent setup, file content transfers, decisions and full worker-result inspection are not newly connected. PR #1 remains draft; main is unchanged.


### 2026-10-18 — Token-free local UI access

At User's request, localhost entry points no longer request a gateway token or send empty bearer headers. Explicit `gateway --local-ui` / `daemon --local-ui` enables direct-loopback human API access; default and remote deployment behavior remains paired. Startup rejects non-loopback binds, configured tunnels and proxy trust. Local requests validate Host/Origin/fetch metadata and reject forwarding headers. TOTP, Guard, policy, root-of-trust and owner isolation remain independent. Main is unchanged and PR #1 stays draft.

Verification: full locked/offline all-target Rust tests pass (five existing host-dependent sandbox checks ignored), including local-mode request rejection and startup restrictions. Frontend build, 14 unit/API tests, formatting and 39 desktop/phone browser checks pass (one intentional skip). Local fixture gateway smoke covers token-free requests and normal paired mode separately, including SSE and persisted owner isolation. No production deployment or live Claude/Guard/external-write acceptance is claimed.


### 2026-10-19 — Product plan and local steps 1–3

User approved the five-step order now maintained in docs/product-plan.md and requested implementation of steps 1–3. Development credentials use the local environment; production web enrollment is planned, not enabled. Created private/git-ignored .env.local and .runtime state, with a launcher that never prints credentials or passes them in argv. After User configured the environment locally, actual Fast/Normal/Deep Claude requests all completed successfully. The real loopback runtime is running on port 8080, serves the built prototype, and retains successful provider evidence after restart. No credential or private response was copied into source/docs/tool output.

The prototype defaults to live gateway data; fictional previews now require mode=sample. Dedicated human settings APIs persist nonsecret global preferences/model routing/budgets and owner soul/memory notes/default profiles/skill text. Schema v8 adds versioned settings and audit scope/revision only; stale saves return 409 and leave drafts intact. Subsequent runtime requests use saved guidance/profiles. Skill scripts never auto-execute; Guard/root-policy/TOTP/tool/secret/peer boundaries remain independent. Matching complete stopped-state backups are required before upgrading existing deployments. Maintained Markdown guides/trackers now point to the current plan; historical checkpoints, upstream instructions and agent identity templates remain preserved.

Validation: actual Claude routing verification; restart health/settings/prototype/status checks; full locked/offline all-target Rust suite 2,442 passed with five pre-existing host sandbox ignores; frontend production build/14 unit tests/formatting; desktop/phone browser suite 41 passed, one intentional skip; paired and token-free fixture gateway smoke including owner guidance, default profile dispatch, revision conflicts and protected management paths. Browser checks caught unstable nested-label names and were corrected with explicit labels before rerunning. The schema-upgrade test was updated from v7 to v8. Local Rust remains 1.99; final pinned CI is separate. PR #1 must stay draft, main unchanged, and steps 4–5 remain open.

Final follow-up: tiny/standard suites each pass 1,716 executions (five host ignores); route verification is profile-specific even when profiles share a model, covered by a new regression check. Full suite total is 2,442. The settings keyboard shortcut and live navigation flows passed four focused browser reruns after the full suite. Rust 1.99 Clippy has inherited diagnostics; the new settings format-allocation diagnostic was corrected, leaving no new settings diagnostics. Pinned Rust 1.92 CI remains the merge gate.


### 2026-10-19 — Step 4 complete locally; step 5 validation

Fetched origin and compared the clean UI branch/upstream before implementation; no incoming commits required integration. User clarified the request as steps 4 and 5 (the plan has no step 6).

Connected the live thread workspace to authenticated file upload/download, durable human-only decisions and full structured worker results with archive downloads. Schema v9 adds immutable decision records; exact UUID/content retries are idempotent, changed content/scope is rejected, audit is recorded once, and decisions never authorize tool execution. Worker filtering retains task/thread ownership. Refresh, partial error, empty/loading and mobile states are exercised in browsers. No raw credential input was added.

Tests caught and corrected a held SQLite mutex during decision-list refresh, a test selector that matched two independent loading statuses, and the connected agent accessible-name assertion. Removed the obsolete duplicate file metadata fetch. Added fixture HTTP binary transfer checks, peer download denial, and complete stopped-tree backup/restore with all-file hash equality, SQLite integrity and restored decisions/files/worker artifacts. The normal paired and token-free loopback smoke both pass. Full Rust suite: 2,445 passed, five existing host sandbox ignores. Frontend production build/14 unit checks pass. Browser suite: 43 passed and one intentional skip. A two-independent-connection bounded-session race confirms one admission against one remaining operation and durable zero budget after reopening.

Inspected previous-head GitHub checks: security audit, supply chain, frontend, browsers, deployment static/image and actionlint passed; required Rust 1.92 failed on GatewayConfig's fourth independent serialized boolean. Added a narrow documented compatibility allowance rather than changing the lint gate. Found and fixed literal backslash-n output in the Linux service-unit renderer; the regression now checks actual section lines/umask. Local Rust 1.99 still has inherited Clippy diagnostics; final pinned CI is separate.

Step 5 is not complete. Asked for target SSH alias/service user/config directory; none confirmed yet. The existing documentation differs between an earlier Debian VM and intended Rocky target, so no host is guessed from unrelated SSH entries. No new live Guard, n8n/TOTP, production web enrollment, service install, deployment or external write was performed. The new acceptance runbook records exact outstanding evidence. PR stays draft and main unchanged.

Final local follow-up: tiny/standard suites each passed 1,719 tests (five existing host ignores); the explicit v8 layout migration retained saved settings. Took a private complete stopped `.runtime` backup at `/private/tmp/raaviclaw-pre-v9-l7hdct5g`, checked copied databases for integrity, and restarted the real local gateway on schema v9. Health, prototype, settings and decisions API passed; prior Claude profile verification records remain present without new model calls. Browser upload acceptance now waits for the enabled input after initial loading. Decision UTF-8 byte limits are validated before creating a pending retry, so oversized Unicode text remains editable.

User replied to the deployment input request: “put a pin on it, too advanced.” Step 5 is now deferred by user request. Stop target-host/live integration work until resumed; no SSH/config details are currently requested. Step 4 and completed local recovery validation remain delivered.


### 2026-10-19 — UI priorities 1–2: consistency and settings discovery

Fetched origin, compared the clean feature branch to upstream/main and found no incoming changes before implementation. User asked to finish UI consistency and settings discovery after sharing the connected workspace screenshot. Applied the apple-design skill to layout, labels, sidebar hierarchy, settings navigation and keyboard access using the existing web-app design tokens.

The single global footer entry now visibly reads App settings with a gear, global-preferences subtitle, tooltip/shortcut and selected state. Owner gears have labels/tooltips/pressed state and still open separate owner setup. Restored the prototype’s settings heading/icons, appearance cards, model shortcut, keyboard hints and Connection & security section. Agent Credentials is visible as a clearly unconnected read-only section; no secrets or fictional records are accepted/shown. Open chat is available from agent settings. Header appearance switching persists through the existing revision-checked human settings API; conflicting saves report failure and reload current preferences without overwriting newer state.

Aligned live forms, toolbar, message/card spacing and activity styling with the prototype. View changes reset content scroll so the newly opened page is visible. The mobile drawer traps Tab focus while open, restores focus on close and is inert/aria-hidden when closed. Sidebar thread scrolling retains a visible settings footer. Visual checks caught automatic scrolling of the outer sidebar during focus changes; overflow clipping keeps its branding/footer stable while the thread list retains its own scroll.

Validation: frontend build and 14 unit/API tests pass; desktop/phone Chromium suite 45 passed, one intentional skip. New checks cover settings discovery, separation, selected sections, appearance persistence/conflict handling, visible branding/footer, scroll reset, mobile focus and overflow. The initial test matched agent/global settings navigation by a substring; it now checks the exact global accessible name. No backend schema/dependency/security capability change was needed. UI-3 through UI-6 remain planned; deployment acceptance remains deferred by User. PR stays draft and main unchanged.


### 2026-10-19 — Retire the old management UI

Fetched origin and compared the clean feature branch to upstream before changing code. User pointed out that the Approvals link still opened the unfinished old root UI. Removed the duplicate 1,525-line management shell and global stylesheet. Both HTML entries now import the same current application; `/` is canonical and `/prototype.html` remains an alias. Approval navigation and the settings shortcut stay inside the current shell. Disconnect now clears the current session and shows Connect, without automatically substituting sample data.

Migrated the unique retained flows into current native components: exact single-action/bounded-session approval review, dedicated TOTP submission, uncertain-authorization blocking, expiry/budgets/results/reconciliation/End session, exact peer question/response review, paginated private/shared memory with filters/edit/verify/stale/delete/exact sharing/revocation, and Activity watch settings/recommendations. Existing backend contracts and authorization.ts are unchanged. No production credentials were entered, no external write dispatched and no target deployment work resumed. Local development model profiles remain Sonnet 5.5.

Validation: frontend production build and 14 unit/API checks pass; desktop/phone browser suite 57 passed and one intentional skip. New fixtures verify both URLs load the same script, approvals do not navigate away, codes only use the dedicated endpoint and clear after uncertain submission, refreshed authority controls retry, session closure/budgets are shown, expired grants cannot authorize, peer responses use exact hashes, memory sharing uses the reviewed hash and owner isolation persists, and watch schedule saves preserve disabled state. Tests caught ambiguous accessible-name queries and the memory dialog’s label association; native dialogs now have explicit titles and text areas are tested by their accessible textbox role. Existing disconnect expectations were updated to the live connection screen. Running-gateway HTTP checks confirm both URLs serve the same built script. Final-head GitHub checks remain separate; main is unchanged and PR stays draft.

## October 19, 2026 — live chat polish

Fetched GitHub and confirmed feature HEAD/upstream matched before editing. Extracted the live conversation/composer into a component that uses existing tokens, portraits and runtime operations. Added a growing bottom composer, compact profile/model display, readable messages, draft-only starter prompts, copy feedback, conditional scroll following/Jump to latest, and position-preserving earlier history. Desktop Enter/Shift+Enter and IME safeguards are retained; touch Enter adds a newline. Interrupted/stopped requests retain drafts and partial text with explicit history recovery, avoiding blind duplicate submission. Stop remains browser cancellation, not a promise that runtime work stops.

Browser checks caught a completion-focus regression caused by the history reload unmount and outdated empty-state assertions; focus is restored after the confirmed history load and assertions now target the new empty state. The first browser invocation lacked the expected default Chromium path; reran with the installed temporary browser path. Build and 14 unit/API checks pass; 67 browser checks pass, one intentional desktop skip. Desktop/phone screenshots inspected. No real provider requests, backend/security changes or external writes. Deployment remains deferred; keep PR #1 draft and main unchanged.

## October 19, 2026 — Phase 0 closed on the Rocky development VM

User requested completing Phase 0 on the Rocky Proxmox development machine. Source under validation was consolidated main `c62e27b`, including implementation tip `1f97292`. Earlier branch consolidation was explicitly authorized: both duplicate feature branches are removed and PR #1 is merged. Updated the current agent instructions and tracker to distinguish that completed consolidation from still-deferred production deployment.

The VM initially lacked Rust, Node and a C compiler. Sudo required an interactive password, so User received the native package command and installed the compiler/browser prerequisites through the terminal. Installed isolated workspace tools without changing shell profiles: Rust/Cargo 1.92.0 with rustfmt/Clippy, Node 22.23.2/npm 10.9.8 and a conda-forge C/C++ toolchain. Added `dev/rocky-run.sh` to select these tools and bound Cargo parallelism to two jobs. Documented bootstrap, checks, local startup/restart and the development binary's local toolchain-library dependency in `docs/rocky-development.md`.

Fresh validation:

- Rust formatting and strict locked all-targets Clippy passed.
- Locked all-targets full tests: 2,445 passed; tiny and standard: 1,719 each. Each tier retains five explicit host-policy/rootless-sandbox acceptance ignores.
- Locked debug and optimized release builds passed. Release binary: 9,387,056 bytes; help/startup passed.
- Paired and token-free debug smoke passed, including owner isolation, persisted work, authenticated events, restart and complete stopped-state backup/restore with matching hashes/SQLite integrity. Token-free smoke also passed against the optimized binary.
- Offline PTY TUI rendering/input/exit passed without provider calls.
- Frontend locked install, production build, 14 unit/API tests, Prettier and dependency audit passed; zero reported vulnerabilities. Chromium desktop/phone checks: 67 passed, one intentional skip.
- Shell syntax and Git whitespace checks passed. Raw local logs are under ignored `.runtime/phase0-evidence/`.

No application source fix was needed. Existing tiny/standard unused-item warnings and Vite's informational chunk-size warning were not changed. Phase 0 is complete for the authorized development scope. No live provider/Guard/n8n call, service installation, external write or production deployment was performed. Production acceptance remains deferred, rather than being silently counted as complete.

## October 19, 2026 — Eight-phase Phase 1 sign-off and Phase 2 hardening

User requested finishing Phases 1 and 2 of the current eight-phase plan. Added `docs/eight-phase-plan.md` to distinguish this grouping from the original P00–P25 roadmap: Phase 1 groups P01–P05 and Phase 2 groups P06–P09. Preserved all existing implementation and earlier uncommitted Phase 0 documentation/tooling changes. No duplicate registry or frontend rewrite was introduced.

Phase 1 is complete locally. Fresh `assistant_isolation` (11) and `work_foundation` (six) integration tests confirm owner isolation, durable tasks/threads, restart/migration behavior, restricted workers/results/archives, decisions and audited task edits. Full feature-tier suites and paired/loopback daemon recovery checks also pass. Worker shell/network/credential grants remain disabled rather than silently inheriting parent privileges.

Phase 2 changes:

- Guard transport rejects missing/ambiguous verdicts and incomplete/filtered completions. Local HTTP fixtures cover SAFE, BLOCK/S14, malformed/oversized JSON, HTTP errors, redirects and timeout. The live endpoint/model and immutable safety categories remain unchanged.
- Canonical UUIDs, bounded scope/serialized payload and control-character checks run before Guard/admission. Human grants/exceptions validate request scope. Regression checks ensure malformed requests do not call Guard or consume valid approval.
- Safer alternatives validate scope/identity, retain the original authorization boundary, re-enter Guard, require a fresh exact human approval and remain single-use. Deep review does not authorize execution.
- Secret redaction covers additional padded/unpadded standard/URL-safe Base64 and uppercase hex in nested values/keys; expiry/revocation/rotation and owner/capability tests remain passing. Arbitrary secret transformations are not claimed to be covered.
- Sandbox output is bounded after lossy UTF-8 decoding; explicit rootless UID/GID mapping supports non-1000 host users and private input mounts request SELinux labeling.
- Added an opt-in real broker/container admission test and `dev/phase2-acceptance.sh` for real isolation, media, orphan recovery and protected host-policy checks. No fixture substitutes for a container or live provider acceptance.

Final local checks: strict fmt/Clippy pass; full all-targets suite 2,457 passed; tiny/standard each 1,731 passed. Seven ignores are the five existing host acceptance tests and the new real broker/container test compiled into both unit-test targets. Debug/release builds, paired/token-free debug recovery smoke and token-free optimized-binary recovery smoke pass. Shell syntax and Git whitespace pass. Evidence logs remain private/ignored under `.runtime/phase0-evidence/phase1-*` and `phase2-*`.

Phase 2 is not signed off: Podman is unavailable in the current VM PATH, no reviewed local sandbox image is available, and protected host-policy acceptance has not run. Provided User the manual native prerequisites and documented `sudo sh deploy/setup-execution-host.sh`; waiting for the terminal installation result. Existing subordinate UID/GID ranges and `/run/user/1000` are present. The VM currently reports SELinux disabled; production enforcing-mode acceptance remains a separate deferred gate. No service, live provider/Guard/n8n call, external write or production deployment was enabled.

## October 20, 2026 — Real local Phase 2 acceptance complete

User supplied successful output from manual `sudo sh deploy/setup-execution-host.sh`. Followed the new-session GitHub check before continuing: `git status --short --branch`, successful `git fetch origin`, and upstream comparison showed zero ahead/behind commits. Main is at `c62e27b`; all earlier local edits and User's updated AGENTS instructions were preserved. No commit/push was performed; this acceptance and hardening remain local-only.

Rootless Podman 5.8.2 reports true, the installed root-owned mode-0644 policy passes runtime validation/default denial, and the current host reports SELinux Enforcing (superseding the previous day's observed state). Built the reviewed sandbox Containerfile locally; immutable manifest: `localhost/raaviclaw-sandbox@sha256:b6007ca91d75d849dadf67de5582e234bd954d4a187ba64fb90139a39755fe85`.

Added `dev/build-package-fixture.py`: a deterministic, dependency-free pure-Python wheel is bundled into a test-only derivative image using a private temporary build context and no build network/pull. The base/runtime image is unchanged. Expanded real acceptance to verify package search/install/import/removal and rejection of an unbundled package. This caught malformed escaped newline/JSON output in package search/removal; corrected it and verified the actual output. Added `.runtime` exclusions to Docker/Podman contexts and the existing Docker-ignore regression list; all 12 build-context checks pass.

`dev/phase2-acceptance.sh` passed all seven distinct opt-in checks:

- Networkless/read-only root and input isolation, UID/capability/no-new-privileges restrictions, actual 512-MiB memory/zero-swap/64-PID/one-CPU cgroup limits, timeout cleanup and bounded output.
- Image preview and short-video filmstrip decoding.
- Orphan removal while preserving live-owner containers.
- Offline bundled-wheel installation/use/removal and missing-package denial; scratch packages remain disposable.
- Exact broker single-use admission, real container dispatch and replay denial after reopening.
- Installed protected default-deny policy validation.

Final strict Rust 1.92 Clippy/formatting, shell syntax and Git whitespace pass. Final full suite: 2,457 passed, eight opt-in execution counts ignored in the normal run (six host tests plus the real broker test duplicated into library/binary); all seven distinct cases were separately executed successfully. Debug build and local token-free daemon/restart/complete-state backup/restore smoke pass. Earlier tiny/standard and optimized-build validation remains recorded in the preceding entry; no fresh final optimized build or frontend change is claimed here. No labeled test containers remained after acceptance.

Phases 1 and 2 of the eight-phase plan are complete for this local development scope. Persistent custom package sets/worker execution grants remain unavailable. Live HF/S14 diagnosis, enabled integration/write/service acceptance and production deployment remain separate gates. `/etc/tinyclaw/security/execution.toml` remains absent: no model capabilities or external writes were enabled. Updated the plan/tracker to reflect completed local acceptance without closing original P06–P09 deployed/live-provider gates.

## October 20, 2026 — Phase 3 local connected-operations acceptance

Completed Phase 3 of the current eight-phase plan for local client/runtime acceptance. Initial and final GitHub fetches succeeded; main at `c62e27b` remains zero commits ahead/behind origin/main. Earlier local work and user changes remain preserved. This implementation and its evidence are uncommitted and unpushed.

Added per-workflow operation allowlists, canonical wire-JSON manifest hashing, authorization scope/hash validation before transport, terminal failed authorization cleanup, secret-safe gateway metadata and dual-credential result redaction. Gateway audit IDs and n8n execution IDs remain separate and durable through projection/restart. Ambiguous dispatches remain uncertain until correlated terminal reconciliation; a retry flag alone never restores authority. Session ending revokes locally first, removes broker tokens and durably reports unconfirmed remote closure in the UI. Loopback tests cover authorization, dispatch, budget persistence, replay denial, reconciliation and closure without live external writes.

The write ledger now has a transactional version-1 migration preserving spent budgets and separating legacy audit correlation. Workflow contracts require `allowed_operations`; old pending bounded-session reviews require fresh reviews because canonical manifest hashes changed. Back up complete stopped runtime state before upgrade and use the matching complete backup for rollback to an older binary.

Final evidence in `.runtime/phase3-evidence`: full Rust suite 2,481 passed; tiny/standard each 1,755 passed; eight host-execution test counts remain opt-in with Phase 2's seven distinct real checks separately passed. Strict Rust 1.92 Clippy, Rust/frontend formatting, Git whitespace, shell syntax, debug/release builds, paired/local runtime recovery and optimized-binary recovery smoke pass. Frontend build and 14 unit/API tests pass; browser acceptance reports 67 passed and one intentional skip.

Updated the eight-phase plan, phase trackers and connected-operations documentation. Live gateway/TOTP acceptance, distributed gateway atomicity/idempotency, real Hugging Face/S14 diagnosis, credential enrollment and production service activation remain explicit release gates. No live n8n operation, external write, model execution capability or production deployment was enabled.

## October 20, 2026 — Phase 4 local source-aware foundation

Checked branch/tree before implementation. The sandbox's initial fetch could not write FETCH_HEAD; the reviewed elevated fetch succeeded, as did the delivery recheck. Main remains at `c62e27b`, zero commits ahead/behind origin/main, with no incoming changes. Preserved existing staged/unstaged work; no commit, push, reset or branch change was performed. All Phase 4 work remains local.

Implemented human-only shared-memory save/verify/stale contracts with owner/exact-review binding, input validation, audited transactions and preserved source linkage. Edits do not silently change private facts. Expired shared facts require an explicit expiry/content edit before verification. Private expiry review marks facts stale and audits the transition idempotently without deletion or expiry extension, including when watches are disabled. Worker candidate promotion now shares the management validator, denying token/credential categories rather than maintaining a divergent category list.

Bound peer question/response review hashes to exchange identity and expiry as well as owner/task/content. Admission recomputes the binding transactionally; legacy/changed grants cannot authorize new delivery and must be recreated/reviewed. Legacy requests remain revocable and historical delivered answers remain retained. New regression coverage exercises identical questions, transplanted hashes, changed expiry, legacy approvals, restart, two-stage review and closed-scope replay denial without private-domain access.

Watch recovery/completion now precedes quiet-hour checks so disabling a schedule or entering quiet hours cannot strand an admitted investigation. Notifications expose run/worker/reserved-budget correlation and structured source evidence. Added owned/idempotent recovery coverage and an authenticated daemon fixture that enables only its temporary schedule, investigates an overdue task with a tool-free/credential-free worker, emits one source-linked recommendation, denies peer notification access, disables that schedule and verifies durable state through complete stopped-tree restoration.

Final frozen-source evidence in `.runtime/phase4-evidence/`: strict Rust 1.92 all-target Clippy and formatting pass; full/tiny/standard suites pass 2,494/1,768/1,768 executions. Each normal suite retains eight opt-in host counts; Phase 2's seven distinct actual host/container acceptance checks remain separate evidence. Debug/release builds, paired and token-free debug daemon/recovery smoke, and optimized-binary token-free recovery smoke pass. Frontend build and 14 unit/API checks pass; repository-local Chromium passes 67 desktop/phone browser checks with one intentional skip. Dedicated management-contract integration tests pass. Git whitespace checks pass; no dependency/schema-version change was required.

Intermediate runner failures were environmental: restricted loopback sockets, an unset repository-local Chromium path and an overlapping feature build temporarily replacing the daemon binary. The final sequential pipeline (`*-final.log`) exits successfully with a full-feature build before every daemon smoke; browser success is in `browser-tests-local.log`. No product failure was suppressed or test weakened.

Updated the current eight-phase plan, detailed ledger and source-aware foundation documentation. Phase 4 is complete for local foundation acceptance; Phase 5 app flows are next. Shared editing UI and full conversational/card integration remain Phase 5 work. Broader external-source monitoring, independent source re-verification, live provider/security acceptance and production deployment remain gated/deferred. No production schedule, model peer permission, execution capability, credential, external write or always-on service was enabled.

## October 20, 2026 — repository-wide Markdown refresh

User requested updating all Markdown files. Reviewed the 40 repository Markdown files, including hidden contributor/review guidance and the new documentation index. Checked status first; the restricted fetch could not write FETCH_HEAD, then the reviewed elevated fetch succeeded. Main/origin/main remain at `c62e27b` without divergence. Preserved staged/unstaged work; no commit, push, branch or application-code change was made in this refresh.

Added `docs/README.md` as the current documentation index. Updated active README, architecture, roadmap, contribution/security guidance, app/development guides, acceptance trackers and changelog to distinguish local Phases 0–4 from next Phase 5 app work, Phase 6 beta and deferred Phase 7 production acceptance. Replaced stale current-plan banners, clarified the completed PR/consolidation versus unpublished phase changes, labeled historical branch/host evidence and separated the old product deployment step 5 from current Phase 5. Corrected the current development OS to Rocky on Proxmox, retained the original LXC/Debian provenance, documented sequential feature-build/smoke requirements and kept write/credential/provider/service activation gated.

Retained original dated results, immutable upstream guidance, permanent-agent identity prompts and avatar-generation records rather than injecting phase banners into runtime/asset provenance. Current Phase 4 protocol guides remain authoritative for their already accurate detailed contracts. No security requirement or agent permission was relaxed.

Validation: scanned all 40 Markdown files for relative inline link targets; zero missing files. Git whitespace checks pass. Application tests were not rerun for this documentation-only refresh; the 2,494/1,768/1,768 Rust, 14 frontend and 67 browser results quoted in the guides are the recorded Phase 4 evidence, not newly executed checks or fresh remote CI. All changes remain local-only.

## October 20 — Phase 5 app work started

Fetched origin successfully; HEAD and origin/main match at c62e27b. Existing staged/unstaged Phase 0–4 work was preserved. Started loopback backend on 8080 and Vite on 5173 using the Rocky tools. Launcher reports no Claude credential, so real provider replies are unavailable in this running development configuration.

Added Files to connected workspace navigation with an owned Focus Thread selector and the existing authenticated upload/download controls. Reused thread file handling while hiding task/decision/worker panels in Files mode. Frontend production build and all 14 unit/API checks pass; the new Files browser check passes on desktop and phone. Changes remain local-only. Phase 5 remains in progress: chat attachment understanding, shared-memory management controls, runtime card envelopes and broader flow/polish acceptance remain open.

## October 20 — shared Claude credential enrollment

Restarted the paired LAN-development backend with the private environment credential; authenticated runtime status confirms credential presence (not provider validity). Added App settings → Claude & models → Claude API password entry with a separate authenticated, bounded, non-caching endpoint. Tokens are encrypted with the existing SecretStore into a mode-0600 file separate from conversational/settings data; responses and audit events contain metadata only. New Claude requests prefer the enrolled credential, with existing environment/config fallback when absent. User-authorized enrollment supersedes earlier UI-preview-only credential limitations; it grants no tool/integration capabilities. Added --paired launcher option so restarts can load private environment credentials without enabling loopback token-free access.

Build, strict library Clippy, 14 frontend unit checks and focused credential/auth/encryption/replacement checks pass. Live endpoint validation rejects invalid input without changing the credential; the LAN frontend responds on localhost:5173. No actual provider request was made to validate billing/account access. Token storage is encrypted at rest; the current development LAN URL uses HTTP. Changes remain local-only.

## October 20 — publication validation

User requested commit/push of the pending work. Fresh fetch shows no incoming origin/main commits. Full Rust all-target suite passes 2,497 executions with eight host acceptance counts opt-in. The initial sandboxed attempt could not bind three fixture listeners; the rerun with localhost binding allowed passes. Frontend build and all 14 unit checks pass. Full desktop/phone suite passed 70 checks with one intentional skip and found one sidebar overflow regression from the new Files destination. Fixed navigation shrink/scroll behavior so App settings remains anchored; all six affected layout/Files/token browser checks then pass on desktop/phone. Rust formatting, changed frontend formatting and whitespace checks pass. Private environment credentials and LAN token were checked against publication candidates and remain ignored; runtime data is excluded. These are local verification results, not GitHub CI or production/live acceptance. Phase 5 remains in progress.

### 2026-10-21 — Phase 5 app completion review increment

Phases: current delivery Phase 5; detailed P16–P20
Status: in progress, implemented flows ready for User review

Work performed:
- Preserved the existing uncommitted chat/Markdown/context-summary/task-receipt work. GitHub was fetched successfully after the sandbox's read-only Git mount required escalation; HEAD and its configured upstream matched (0 ahead, 0 behind). All subsequent changes remain local and uncommitted.
- Added Conversation and Files & evidence tabs, keyboard navigation, persistent tab drafts/previews, thread-summary editing, independent evidence refresh and composer attachment entry. Main-chat attachment preparation explains the Focus Thread handoff and preserves its draft when creating the thread.
- Added authenticated bounded text/Markdown/raster previews, checksum/source inspection and explicit text-to-draft inclusion. Image signatures are checked; SVG/HTML are not rendered as images. Uncertain uploads block immediate retry until refreshed file evidence is available. No automatic model file/image understanding is claimed.
- Connected the existing closed card schema to completed assistant replies, with Markdown/code fallback for invalid cards and owner-checked thread navigation. Added the closed card vocabulary to runtime guidance without granting capabilities. User messages cannot create interactive cards.
- Added on-demand task history/worker findings/source panels and recorded watch investigation/evidence/reservation details. Shared-copy edit/verify/stale controls remain sharing-owner-only and exact-hash-bound.
- Added human-prepared peer questions and scoped responses with separate exact reviews, approved-question revocation, closure feedback and explicit recovery after ambiguous creation. Kept dedicated TOTP/write-session controls, and prevented old status reads overwriting newer session results.
- Restored the New thread form and header refresh/recovery entry. Protected the context summary from malformed secondary results; excluded expired reviews, included pending peer reviews and displayed configured daily budgets separately from lifetime usage. Kept phone header controls compact.
- Added `docs/phase5-review.md`, updated the current phase tracker and UI scope. No schema, credential, host-policy, execution-capability, watch schedule or peer-model-grant changes were made.

Validation:
- `npm run build` and `npm test`: production build and all 18 frontend unit checks pass.
- Desktop/phone Playwright: 84 passed, one failed upload test and one intentional skip in the complete final app run. The upload test selected a disabled control before initial loading finished; corrected it to wait for the enabled input. Both desktop/phone targeted reruns pass. Coverage therefore passes all 85 applicable distinct browser cases, with one intentional skip; no unresolved application failure remains. Full-run screenshots/traces are in ignored `app/test-results/`; the corrected upload rerun is in `.runtime/phase5-evidence/upload-rerun/`.
- Rust 1.92: locked/offline all-target tests, formatting, strict all-target Clippy and debug binary build pass. Existing opt-in host acceptance tests retain their ignored status; they were not reactivated. An initial sandboxed fixture test could not bind loopback; the authorized unsandboxed full suite passed.
- Frontend Prettier and `git diff --check` pass. Inspected desktop conversation-card/approval and phone recommendation screenshots. The existing local app responds HTTP 200 at port 8080 and serves the rebuilt frontend.
- No live provider call, n8n operation, external write, service installation or production deployment was performed. The backend was rebuilt, but the existing local runtime was not restarted; its next normal restart activates the new card guidance.

Findings / decisions:
- Phase 5 remains incomplete: native main-conversation attachment/message linkage and multimodal provider image understanding, the durable pending execution/Guard-exception review queue/UI, and complete thread/task list pagination remain open. Existing thread-based attachments, dedicated TOTP approvals and human peer-exchange contracts were reused without introducing authority paths.
- The frontend bundle retains a nonfatal size warning. Visual approval remains User's next step; Phase 7 remains deferred.

Next step:
- User reviews the normal app using `docs/phase5-review.md`; incorporate feedback before deciding the next remaining Phase 5 slice. Commit/push only when requested.

### 2026-10-21 — One-command LAN development launcher

Status: complete locally, uncommitted

- `sh dev/rocky-run.sh python3 scripts/dev-local.py` now builds Rust and the GUI, starts the loopback gateway on 8080 plus LAN Vite on 5173, prints LAN URLs and a fresh terminal-only development access key, and supervises/stops both process groups. Occupied ports fail without killing an existing server. `--no-build` skips builds; `--local` retains token-free loopback-only operation; previous `--build`/`--paired` options remain accepted.
- Added explicit `gateway --dev-access` CLI bootstrap: loopback/no-tunnel only, forces pairing, disables token-free UI for that run, retains existing paired tokens and adds only a hash of the fresh random key to in-memory configuration. No raw key is persisted. The GUI requires the key even at localhost in LAN development mode. Provider credentials are passed only to the gateway, not npm/Vite. The current running development server was not stopped or replaced.
- Validation: three mocked launcher checks; isolated actual gateway fixture verifies anonymous/invalid-key rejection, fresh-key acceptance, no raw-key persistence and non-loopback denial; a real Vite/Chromium fixture verifies localhost key login before private API reads. Frontend build/all 18 unit checks, Rust build/check/strict binary Clippy and formatting pass. No real credential was printed, provider invoked, live external action taken or firewall/service changed. Temporary fixture key material remained in test-process memory.

### 2026-10-21 — Frontend/backend connection, safety and reliability audit

Status: completed locally for the tested development flows; live release gates remain open

Work performed:
- Audited the current uncommitted UI/API, gateway authentication, file, settings, memory, review and LAN-launch paths without resetting prior work or changing live runtime data.
- Reproduced and repaired SSE completion waiting on EOF, an oversized delimited-event limit bypass and unbounded aggregate deltas. JSON/transfers now have 45-second deadlines and chat has a ten-minute ceiling; aborted/unknown mutations retain conservative recovery with no automatic replay.
- Added runtime validation of core list/page read contracts. Malformed reviews fail into recoverable errors instead of crashing or rendering TOTP controls. Backend validation remains authoritative; validation of every settings/session/detail extension is not claimed.
- Added gateway-wide no-store response headers and explicit Vite app-root/credential-path filesystem restrictions. A synthetic private-file marker is denied; harmless SPA HTML fallbacks are distinguished from file exposure.
- Repaired the launcher's dangling-config-symlink write ordering, restricted GUI child environments to needed variables and bound skipped-build reuse to nonsecret HTML login-mode metadata. Mismatched/legacy builds request a full rebuild instead of silently showing the wrong connection screen. All five launcher regressions pass.
- Added `scripts/test_ui_backend.py` and `app/e2e/ui-backend-contract.mjs` for a real browser → Vite → Rust audit with temporary state and a loopback model. Verified actual UI task/thread/file/decision persistence, selected Fast routing, chat history, settings conflicts, owner denials, exact shared edits and peer review/closure, failed missing-review TOTP, credential non-storage/non-injection, reconnect and lost-confirmation recovery without replay. No production provider or external integration is used.
- Created `docs/frontend-backend-audit.md`, updated UI/launcher guidance and corrected earlier documentation: thread/task lists currently return all owned records, not a bounded first page of 50.

Validation:
- Frontend production build and all 25 unit tests pass; frontend formatting passes. The final desktop/phone browser run and its count are recorded below when complete.
- Rust 1.92 locked/offline all-target suite: 2,499 passed and eight existing opt-in host execution counts ignored. Strict all-target Clippy, formatting and the debug build pass. Local log: `.runtime/connection-audit/rust-tests.log`.
- Both paired and direct-loopback full assistant smoke suites pass, including invalid origin/host/forwarding rejection, restricted workers, budgets, source-aware memory/watch/peer behavior and complete stopped fixture-state backup/restore with matching hashes/SQLite integrity.
- Real paired browser/proxy/backend audit passes, including a dropped browser response after the backend has committed and a synthetic private-file probe. Fixtures remove their state/probe afterward; existing live development processes were not interrupted. All bearer/TOTP/provider values in these fixtures are fictional; minted development-key tests keep keys only in subprocess memory.
- One automatic approval review timed out before executing a combined final test-preparation command. Preparation proceeded within the workspace sandbox; the narrowed fixture-only retry was approved and passed. No task remains blocked by that timeout.

Remaining limitations / next step:
- LAN HTTP does not encrypt gateway keys/TOTP/API enrollment. Use trusted development networks; production HTTPS/protected access and live Guard/n8n/TOTP replay/reconciliation remain separate acceptance gates.
- Physical phone/iOS/Safari and large-dataset pagination/load acceptance are unverified. Phase 5 image/native-main-chat attachments, pending execution-review queue and memory expiry editor gaps remain as documented.
- Changes are local and uncommitted. Restart the normal development command to activate the rebuilt backend, then review the audit report before choosing subsequent work.

Final audit validation update: the final built desktop/phone suite passed **89
checks with one intentional skip**. Results are retained in
`.runtime/connection-audit/browser-tests.log`. All 25 frontend unit checks, five
launcher regressions, 2,499 Rust tests, strict checks, both smoke modes and the real
frontend/proxy/backend contract audit pass. No unresolved failure remains in the
executed suites; the report's live/transport/feature limitations remain open.

### October 21 — frontend/backend audit follow-up

Checked `git status`, fetched origin successfully, and confirmed HEAD/upstream had
zero incoming/outgoing commits before editing. Preserved the uncommitted Phase 5
work. Corrected an audit-report error: the API/model task/thread list actions take
50 rows, while their underlying database helpers return all rows.

Added owner-scoped, database-bounded task/thread pages and explicit Load more
controls, including peer-request task selection. Older selected threads resolve
through an owner-checked lookup after refresh. Partial task summaries/calendars
state that older tasks/deadlines remain unloaded. New pagination regression walks
125 tasks and threads, excludes peer rows, checks cursor progression and rejects
invalid limits. It caught a mutex lifetime issue, fixed before delivery.

Expanded runtime response validation to nested task evidence, session budgets and
correlation fields, complete settings/status, record creation/edit acknowledgements
and dedicated write-authorization replies. Invalid replies do not replay writes.
A failed session refresh clears old displayed status. Browser regressions cover
malformed evidence/settings/session budgets without a renderer crash or fabricated
grant. Private/shared memory forms now allow explicit RFC 3339 expiry editing or
removal, with separate verification and existing backend owner/hash checks.

Validation: frontend build and 27 unit tests passed; strict all-target Clippy,
Rust formatting and frontend formatting passed. All-target Rust tests passed:
2,501 successes, eight existing opt-in ignores, zero failures. The initial
sandboxed full Rust run could not bind three fixture listeners; the same suite
passed with loopback permission. The actual paired browser/Vite/Rust fixture
passed, now including expired-memory rejection, explicit expiry removal and
verification, and shared expiry editing without changing the private source.
The final fixed-build browser run is recorded in the audit report once complete.

No live service was stopped/restarted, no live credentials/providers or n8n writes
were exercised, and nothing was committed or pushed. HTTPS/live deployment and
separate Phase 5 features remain explicitly outside this local audit closure.

Final fixed-build browser acceptance: 97 passed, one intentional skip, zero
failures. An earlier overlapping frontend rebuild caused one empty-page harness
failure; the final run kept build output fixed and passed. Follow-up browser/Rust
logs are retained under ignored `.runtime/connection-audit/followup-*.log`.


## 2026-10-21 — Public repository preparation (local only)

Fetched origin successfully after the restricted fetch could not write FETCH_HEAD;
feature HEAD matched its upstream. Preserved all pre-existing staged application
work and did not commit, push, rewrite history or change repository visibility.
Prepared GPL-3.0-only licensing with unchanged inherited MIT attribution, generic
user/project/host examples, broader Git/build/upload exclusions, fork contribution
and vulnerability links, and checksum-verified redacted Gitleaks CI. Release and
container packaging now retain license notices. README review remains next.

Gitleaks 8.30.1 checked shareable current files and all 255 reachable commits;
one exact inherited dummy encryption fixture was allowlisted, then both passed.
A personal author email and earlier identifying content remain in history; a
legacy task-status identifier remains for compatibility. No raw secret findings
or runtime state were copied into repository artifacts. See
[publication review](docs/publication-review.md) for remaining publication decisions.

Validation: 27 frontend unit tests, production build, 33 assistant library tests,
16 desktop/phone settings/calendar checks, Rust formatting/strict library Clippy,
affected frontend formatting and 15 ignore checks passed. Initial test attempts
hit sandbox socket restrictions and a missing matching Chromium browser; reruns
passed with loopback access and a browser installed in ignored runtime storage.
Workflow YAML and TOML/JSON metadata parse; container/release jobs were inspected,
not executed. All cleanup is local and the existing staged index is unchanged.

## 2026-10-21 — Chat/task screenshot follow-up

Fetched origin and confirmed the feature branch matched upstream before this work.
Preserved and included the existing application/audit and publication-preparation
changes at the user's explicit request to commit and push all current work.

Legacy internal-action envelopes now render a request-status message in saved
history and partial replies, without suggesting execution succeeded. A failed
chat stream refreshes runtime tasks independently, so completed internal actions
can appear in the context rail even when the final reply is unconfirmed. Failed
workspace/task refreshes no longer display old tasks as current. The composer
still requires a history refresh after an uncertain response; no automatic replay
or duplicate task creation is introduced. Existing staged runtime receipts and
stream filtering changes are retained.

Validation: frontend build and all 28 unit tests passed. Added desktop/phone
browser regression coverage for legacy payloads, task refresh after a stream
error, draft preservation, and explicit refresh before retry. The first browser
run exposed an incorrect accessible-name selector in the new test; corrected it.
The staged secret scan passed. Rust checks could not be rerun in this session
because cargo/rustc are absent from the execution environment; earlier checks
are documented above and do not constitute a fresh run. No live runtime was
restarted and no external workflow write was executed.

Final targeted browser run: 12 passed across desktop and phone, zero failures.

Full fixed-build browser suite: 99 passed, one intentional skip, zero failures.
Six launcher unit tests passed. A broad Python unittest-discovery attempt also
imported standalone socket-based integration scripts and hit sandbox permission
errors; those integration scripts were not rerun against the prebuilt binary,
which would not establish validation of the current Rust source.

## 2026-10-21 — Expired-provider-credential diagnosis and guidance

Fetched origin; HEAD matched upstream. Recent runtime metadata showed immediate
failed model attempts rather than timeouts or successful replies. User confirmed
that the provider key had expired. The previous change addressed stale task and
protocol presentation but did not resolve this authentication failure.

Preserve typed provider HTTP status through inference context. Return fixed
failure codes in the chat stream and audit, and map them to fixed frontend
instructions. Authentication failures now identify the rejected credential and
point to App settings → Claude & models. Provider bodies, URLs, and credential
values are excluded from the new diagnostics. Unknown errors keep the existing
unconfirmed-response flow, and all failures still require history review before
retrying to avoid repeating an earlier internal operation.

A proposed direct provider probe was rejected by automatic approval review because
it would have reused a process-environment credential. It was not executed or
retried. User's expiry diagnosis made that probe unnecessary. No credential was
read into model context or changed, and no live provider request was made here.

Validation: frontend build, all 29 unit tests, formatting and diff checks passed.
Added Rust coverage for HTTP-status classification through wrapped errors and
unknown-error redaction; Cargo/rustc remain unavailable in this environment, so
Rust tests/build could not be run. Live gateway was not restarted; the backend
change requires rebuilding and restarting before the new failure code is served.
Final desktop/phone recovery and smoke run: 25 passed, one intentional skip,
zero failures. Includes rejected-credential guidance, no provider-body display,
draft preservation, current task refresh and explicit recovery before retry.

## 2026-10-21 — Global task visibility after Focus Thread selection

Fetched origin and confirmed HEAD matched upstream. The screenshot's selected
"test" thread exposed a hidden frontend filter: global Tasks applied the current
chat thread ID even when its status dropdown said "All tasks". The context rail
used all loaded owner tasks, producing an apparently contradictory empty list.

Global Tasks now lists the selected agent's tasks across all Focus Threads and
unlinked tasks, respecting only its visible status filter. Focus Thread workspace
lists retain their thread scope. The global New task form defaults to No thread
instead of silently inheriting the last-selected chat thread; users can explicitly
choose a thread. No backend or database changes are involved.

Validation: production build and 29 unit tests passed. Added desktop/phone coverage
for selecting a thread before opening Tasks, visibility of unlinked/other-thread
tasks, All tasks including completed work, and the new-task thread default. The
initial regression run used an incorrect thread-button accessible name; corrected
the test selector to include the rendered # prefix.
The new test also needed a non-exact select-label match because HTML labels
include option text. Final browser run: all 14 passed on desktop and phone.

## 2026-10-22 — Explicit active-conversation routing context

Fetched origin and confirmed HEAD matched upstream. The active Focus Thread's
history was correctly selected, but its identity was not included in the model
context. Relevant-thread retrieval could then show that same thread as a related
destination, explaining the response offering to save information "there" while
already inside that thread.

Chat preparation now includes host-selected conversation kind, owner, and active
thread ID/title/summary, explicitly distinguishing current Focus Thread from main
chat. Thread titles and summaries remain untrusted data. Related-thread retrieval
excludes the active conversation, and thread-reference cards may use the runtime's
current owned thread ID. Added regression coverage for active identity, main-chat
null thread ID, isolated histories and rejection of a different owner's thread.

Correction to earlier tooling notes: a repository-local Rust toolchain exists
under ignored .runtime/dev-tools; it was absent from PATH and missed by the earlier
limited-depth searches. Using it for backend validation now. Rust formatting also
normalizes the previous credential-diagnostic logging statement. No credentials,
agent ownership or private-memory access rules were changed.

Validation so far: current-conversation regression passed; cargo fmt --check,
strict all-target Clippy and diff checks passed. Full Rust suite includes the
previous provider-status diagnostics tests that had not been runnable from PATH.
The first full run passed library, binary and assistant acceptance tests, then
stopped on an obsolete packaging assertion requiring LICENSE to be excluded.
The earlier licensing cleanup deliberately includes LICENSE, LICENSE-MIT and
NOTICE for Dockerfile packaging. Updated that test to require all three notices
instead; production exclusions were not relaxed. Rerunning the full suite.
Final full Rust run: 2,507 passed, eight opt-in ignores, zero failures. Includes
active-conversation regression, owner isolation, and provider failure diagnostics.
Strict all-target Clippy and Rust formatting passed. Live gateway remains running
its existing process; rebuilding/restarting is required to activate backend changes.
The locked development binary build also passed. The binary is ready locally;
the existing live gateway process was not restarted and must be restarted to load it.

## 2026-10-22 — Public source preparation and main consolidation

Fetched origin before review: feature HEAD `ea72d4d` matched its upstream and was
13 commits ahead of origin/main with no incoming commits or uncommitted changes.
User authorized main consolidation and README updates, chose a separate public
repository with clean history, and selected the name Clawberry. Preserve the
original private repository and all contributor commits. Publication visibility
remains a final review step after preparation.

Rewrote README and contributor onboarding for an early-stage community source
release; added contributor starting points and corrected stale security claims.
Fixed inherited Docker/release paths that expected the nonexistent zeroclaw
executable, pinned release toolchain/locked builds and retained license notices.
Compose now builds this checkout, persists the actual container data path and
binds the published port to loopback; it does not bundle the React app.

Fresh remote security checks identified RUSTSEC-2026-0382 and RUSTSEC-2026-0381.
Updated only rustls-webpki (0.103.16), webpki-roots (1.0.10), their lockfile records
and the direct roots minimum; no advisories were suppressed. Retesting the patch.

Initial evidence: Gitleaks 8.30.1 passed current tracked sources and all 259 fetched
commits; no ignored tracked files. Frontend build and 29 tests passed. Browser
suite passed 101 with one intentional skip. npm audit returned zero vulnerabilities.
Rust formatting/strict all-target Clippy passed and the pre-patch full Rust suite
passed 2,507 executions with eight opt-in ignores. The first restricted Rust/browser
runs could not bind local fixture sockets; they were rerun with loopback permission.

Patched dependency validation: full locked/offline all-target Rust suite passed
2,507 executions, eight opt-in ignores, zero failures; strict all-target Clippy
and Rust formatting passed. Workflow/Compose YAML parses and diff checks pass.
Created the separate Clawberry repository privately for clean-import review;
no existing repository visibility was changed. Frontend source behavior is
unchanged by this preparation. Final remote dependency/security and packaging
checks still need the exact prepared commit.
