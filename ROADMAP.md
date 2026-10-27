# ROADMAP.md

Current eight-phase numbering and acceptance: [eight-phase delivery plan](docs/eight-phase-plan.md).

Current delivery status — October 20, 2026: [eight-phase plan](docs/eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

Current release sequence and October 19 branch consolidation: [release roadmap](docs/release-roadmap.md). Main now includes the consolidated backend and frontend implementation; deployment acceptance remains deferred.

## Goal

Build a self-hosted TinyClaw fork that combines TinyClaw's lightweight agent runtime with Muse-style persistent assistants, proactive monitoring, subagents, memory, tool use, and a clean app experience.

Primary permanent agents:

- Janet — work/business
- Harley — personal

User remains the authority.

Deployment update (2026-10-06): User selected one Proxmox VM, four cores and 6 GB RAM, replacing the original nested LXC target. Keep remote inference, bounded concurrency and task sandbox limits; measure memory use during VM acceptance. See [current acceptance status](docs/p06-p20-progress.md).

---

# Guiding Build Strategy

Do not rewrite TinyClaw wholesale.

The sequence should be:

1. understand and stabilize upstream
2. add multi-agent identity
3. add tasks and subagents
4. add security/control layers
5. add sandboxed execution
6. connect n8n/TOTP
7. add proactivity
8. add memory
9. add the chat-first app
10. refine and harden

Each phase should leave the system usable.

---

# Phase 0 — Fork, Baseline, and Inventory

## Objective

Create a clean fork and understand exactly what TinyClaw already provides.

## Work

- fork upstream repository
- establish `main`
- create feature-branch workflow
- build TinyClaw unchanged
- run it on the target LXC
- document startup/service management
- document current configuration
- inspect:
  - agent runtime
  - provider abstraction
  - channel abstraction
  - tool abstraction
  - memory/storage
  - gateway
  - daemon
  - security policy
  - secret handling
- identify what can be extended vs replaced
- add baseline tests
- capture current DB schema
- add `AGENTS.md`
- add `ARCHITECTURE.md`
- add `ROADMAP.md`

## Deliverables

- fork builds successfully
- systemd/service deployment works
- current upstream behavior is reproducible
- architecture inventory exists
- no behavior changes yet

## Exit Criteria

```text
✓ build passes
✓ daemon starts
✓ existing CLI/TUI works
✓ baseline tests pass
✓ target LXC deployment documented
```

---

# Phase 1 — Permanent Multi-Agent Foundation

## Objective

Introduce Janet and Harley as true independent permanent agents.

## Work

- agent registry
- permanent-agent configuration
- Janet identity/config
- Harley identity/config
- isolated conversation state
- isolated workspaces
- isolated memory references
- per-agent model profile
- per-agent tool permissions
- per-agent secret permissions
- internal message envelope
- basic runtime routing by agent ID

## Initial structure

```text
agents/
├── janet/
│   ├── AGENT.md
│   └── config.toml
└── harley/
    ├── AGENT.md
    └── config.toml
```

## Deliverables

You can independently run:

```text
tinyclaw chat janet
tinyclaw chat harley
```

or equivalent API calls.

## Exit Criteria

```text
✓ Janet context does not leak into Harley
✓ Harley context does not leak into Janet
✓ separate workspaces
✓ separate histories
✓ separate permission identities
✓ same daemon can host both
```

---

# Phase 2 — Tasks + Focus Threads

## Objective

Move from "chat sessions" to durable work management.

## Work

- task schema
- task lifecycle
- task ownership
- task events
- task-to-agent association
- Focus Thread schema
- thread/task association
- thread files
- thread artifacts
- compact thread summary
- main-chat relevance hooks

## Deliverables

Janet can create:

```text
Focus Thread: Example Project Permit
Task: Investigate current city comments
```

without cluttering the primary main chat.

## Exit Criteria

```text
✓ task persistence
✓ thread persistence
✓ main chat remains separate
✓ compact thread summary available
✓ no automatic archival
```

---

# Phase 3 — Temporary Subagents

## Objective

Let Janet and Harley delegate real work to ephemeral workers.

## Work

- `agent.spawn`
- parent/child relationships
- scoped task context
- scoped tool permissions
- scoped filesystem permissions
- model profile per worker
- budget per worker
- worker status
- `agent.wait`
- cancellation
- completion result format
- archive on completion

## Suggested first worker roles

- researcher
- coder
- document reviewer
- video analyst

## Deliverables

Example:

```text
Janet
→ spawn research worker
→ worker completes
→ Janet receives structured result
→ worker becomes archived
```

## Exit Criteria

```text
✓ worker is temporary
✓ parent can inspect status
✓ parent receives result
✓ logs retained
✓ worker process/session removed after completion
```

---

# Phase 4 — Task Archive + Memory Candidate Pipeline

## Objective

Separate logs from long-term knowledge.

## Work

- task archive format
- transcript archive
- shell log archive
- tool call archive
- source list
- cost summary
- artifact storage
- memory candidate output
- parent review of memory candidates
- promote/reject operations

## Exit Criteria

```text
✓ full historical task is inspectable
✓ no raw log dump into long-term memory
✓ useful fact can be promoted
✓ source/confidence retained
```

---

# Phase 5 — Model Router

## Objective

Create provider-agnostic effort profiles.

## Profiles

```text
fast
normal
deep
```

Initial mapping:

```text
fast   → Claude Haiku tier
normal → Claude Sonnet 5.5
deep   → Claude Opus 5.5
```

## Work

- routing abstraction
- provider/model aliases
- cost tracking
- token usage
- per-task model override
- escalation API
- retry escalation
- budget handling

## Deliverables

Agents request:

```text
reasoning.escalate(profile="deep")
```

instead of model IDs.

## Exit Criteria

```text
✓ provider can be changed without editing agent prompts
✓ Sonnet is default
✓ Haiku handles light tasks
✓ Opus can be called selectively
✓ usage/cost logged
```

---

# Phase 6 — Root of Trust + Policy Engine

## Objective

Establish non-negotiable safety boundaries before broad shell access.

## Work

- root-policy config
- immutable deny operations
- deterministic action classification
- approval types
- restricted system resources
- cross-agent access denial
- audit-protection rules
- secret-protection rules

## Hard Deny Examples

- disable Guard
- disable policy engine
- export secrets
- tamper with audit DB
- tamper with security DB
- grant unrestricted permanent host root
- self-authorize approval
- directly read other permanent agent DB

## Exit Criteria

```text
✓ hard deny cannot be overridden in UI
✓ hard deny cannot be overridden by Opus
✓ runtime account cannot rewrite root policy
✓ denied actions are audited
```

---

# Phase 7 — Llama Guard + Opus Review

## Objective

Add intelligent blocking and escalation.

## Work

- Hugging Face Inference Providers integration
- `meta-llama/Llama-Guard-4-12B`
- Guard request schema
- Guard response normalization
- tool/shell pre-check
- blocked-action escalation
- Opus review prompt/schema
- safer-alternative generation
- approval recommendation
- re-run safer action through Guard

## Required Flow

```text
action
↓
root policy
↓
deterministic policy
↓
Llama Guard
↓
SAFE → execute
BLOCK → Opus
          ↓
       deny
       safer path
       or User approval
```

## Exit Criteria

```text
✓ Guard may hard-block
✓ Opus cannot silently bypass Guard
✓ safer alternative can be retried
✓ User receives scoped approval request when needed
```

---

# Phase 8 — Secret Broker

## Objective

Keep credentials out of model context.

## Work

- encrypted secret storage
- secret IDs
- capability metadata
- per-agent scope
- per-tool scope
- subagent secret grants
- runtime injection
- output redaction
- audit logging
- rotation support

## Example

Agent sees:

```text
credential: pricing_api
```

Tool executes with:

```text
X-API-Key: <secret>
```

but model never receives `<secret>`.

## Exit Criteria

```text
✓ raw value absent from model messages
✓ raw value absent from normal logs
✓ permission checked before use
✓ secret usage audited by identifier
```

---

# Phase 9 — Sandboxed Shell

## Objective

Give agents real operational capability without granting unrestricted host root.

## Work

- shell broker
- per-agent sandbox
- per-task workspaces
- filesystem scoping
- network policy
- command audit
- process timeout
- resource limits
- package manager abstraction

## Initial Commands

```text
shell.exec
shell.read_file
shell.write_file
shell.list
package.search
package.install
package.remove
package.list
```

## Exit Criteria

```text
✓ ffmpeg install works in sandbox
✓ yt-dlp install works in sandbox
✓ agent cannot alter root policy
✓ agent cannot inspect raw secrets
✓ agent cannot directly read other permanent agent DB
```

---

# Phase 10 — Media/File Workflows

## Objective

Prove useful autonomous local execution.

## Reference Workflow

Video review:

```text
Harley
→ receives video or URL
→ spawns video worker
→ checks/install yt-dlp
→ downloads video if authorized
→ checks/install ffmpeg
→ extracts frames
→ builds filmstrip/contact sheets
→ inspects visual evidence
→ returns summary + artifacts
```

## Work

- file ingestion
- image ingestion
- artifact storage
- thumbnail generation
- filmstrip generation
- metadata extraction
- attachment previews

## Exit Criteria

```text
✓ agent can process uploaded file
✓ artifacts linked to task/thread
✓ results viewable in app/API
✓ temporary files cleaned according to policy
```

---

# Phase 11 — n8n Bridge

## Objective

Make n8n the operational integration layer.

## Work

- authenticated TinyClaw → n8n calls
- request ID correlation
- agent ID correlation
- task ID correlation
- read workflow calls
- write workflow calls
- execution polling/status
- normalized errors
- workflow timeout/retry
- audit ID storage
- inventory existing gateway contracts; use the email-management request/authorize/action/end flow as a reference without treating it as an email-send workflow
- broker-owned webhook credentials and session tokens; return only safe references to agents
- map gateway request/audit/session/action IDs and n8n execution IDs to agent/task/approval records

Keep the bridge read-only until Phase 12 passes. The supplied email-management export demonstrates an existing deployment pattern, not completion of this phase in RaaviClaw.

## Exit Criteria

```text
✓ Janet can invoke read workflow
✓ Harley can invoke allowed read workflow
✓ failure propagates clearly
✓ n8n audit ID maps back to TinyClaw task
```

---

# Phase 12 — TOTP Write Boundary

## Objective

Ensure all external writes remain under explicit authorization.

## Work

- write classification
- distinguish single-action approvals from bounded capability-session approvals
- single-action approval binds the exact operation, target, payload, expiry, and one execution
- session review shows purpose, fixed manifest/hash, allowed resources/actions, duration, and per-operation budgets
- dedicated authenticated TOTP handoff; codes/tokens excluded from models, chat history, execution logs, analytics, and artifacts
- broker-held session token injection; bind grants to agent/task/authenticated broker identity without automatic subagent inheritance
- implement request → review → authorize → action → end; reference email-management sessions have a five-minute maximum
- enforce resource scope on every action; purpose text alone is not a constraint, and the reference gateway's fixed capability set needs explicit resource/task binding for this runtime
- atomic budget reservations, single-action consumption, TOTP replay protection, expiry, and revocation
- keep policy/Guard/root-of-trust checks and permanent-agent boundaries in force for session actions
- reconcile timeout/verification failures before retry; add action idempotency where supported
- durable approval/manifest/session/action/execution/result audit correlation

The reference gateway uses workflow static data and retains only a rolling audit window. Before enabling runtime writes, verify concurrency and restart behavior and use transactional shared state for authorization/replay/budgets where needed; queue mode or multiple replicas require it. Preserve a durable runtime audit independently of the gateway's rolling history. Sharing a TOTP secret across workflows also requires a shared replay ledger.

## Exit Criteria

```text
✓ no supported external write bypasses TOTP
✓ failed/expired TOTP blocks write
✓ read flows remain autonomous
✓ write result correlated to approval
✓ reviewed single-action payload changes invalidate authorization
✓ one TOTP permits multiple actions only within the reviewed session scope and budgets
✓ agent/task/resource mismatch, budget exhaustion, expiry, and revocation deny new actions
✓ concurrent requests cannot double-consume authorization or exceed budgets
✓ TOTP codes, webhook credentials, and session tokens never reach models or retained execution data
✓ uncertain writes are reconciled before replay; completed actions remain auditable after session end
✓ reference mailbox-management permissions do not authorize sending or deletion
```

---

# Phase 13 — Janet and Harley Heartbeats

## Objective

Make permanent agents proactive.

## Work

- per-agent scheduler
- configurable heartbeat intervals
- source checklist
- lightweight checks
- investigation trigger
- task creation
- automatic subagent spawn
- evidence collection
- recommendation builder
- notification classification
- quiet-hours support

## Behavior

```text
detect
→ investigate
→ spawn if useful
→ gather evidence
→ recommend
→ notify User
```

Not:

```text
detect
→ silently write to external system
```

## Exit Criteria

```text
✓ Janet notices stale work
✓ Harley notices personal commitments
✓ both can create investigation tasks
✓ both can spawn workers
✓ no heartbeat write bypasses approval
```

---

# Phase 14 — Long-Term Memory

## Objective

Add durable, source-aware memory.

## Work

- `janet.db`
- `harley.db`
- `shared.db`
- memory schema
- confidence
- provenance
- sensitivity
- visibility
- auto-promotion rules
- excluded sensitive categories
- search
- inspect
- edit
- delete
- verify
- stale-memory detection

## Exit Criteria

```text
✓ Janet remembers work facts
✓ Harley remembers personal facts
✓ each memory has source/confidence
✓ User can inspect/edit/delete
✓ sensitive categories excluded by default
```

---

# Phase 15 — Cross-Boundary Approval

## Objective

Allow Janet and Harley to collaborate only when User explicitly permits it.

## Work

- cross-agent request object
- UI/API approval
- one-time scope
- task scope
- limited response scope
- exchange audit
- automatic closure

## Exit Criteria

```text
✓ Janet cannot directly ask Harley without approval
✓ Harley cannot directly ask Janet without approval
✓ approval can be single-use
✓ only approved information is exchanged
```

---

# Phase 16 — Chat-First App Foundation

## Objective

Create the primary user-facing experience without building UI primitives from scratch.

## Stack

Preferred:

- Next.js / React
- shadcn/ui
- assistant-ui
- assistant-ui/tool-ui

Optional:

- Refine for CRUD-heavy admin views

## Main Navigation

```text
Janet   [Agent settings]
Harley  [Agent settings]
────────
Focus Threads
Tasks
Approvals
Files
Activity
────────
User   [Global app settings]
```

## Exit Criteria

```text
✓ desktop-first
✓ responsive on phone
✓ Janet main chat
✓ Harley main chat
✓ streaming responses
✓ file/image upload
✓ no dashboard-heavy home screen
```

---

## App and agent settings (confirmed 2026-10-18)

Global app settings and agent setup are separate scopes. Open global settings from the User profile footer or Command/Control + comma; do not duplicate this entry in workspace navigation. Global settings contain appearance, notifications, calendar preferences, gateway connection, shared Claude API configuration and shared Fast/Normal/Deep profile definitions.

Open agent setup from the control beside Janet or Harley. One page per agent consolidates identity/personality and selected memory source files, skills, credential references and access boundaries. Do not scatter these across global settings, a second personality form, or standalone workspace Skills/Vault pages. Project/conversation Files stays separate from agent identity configuration. Shared API/profile configuration does not grant agent capabilities or cross-agent access.

The normal workspace now uses live gateway data and schema-v8 backend settings. Global profile/budget and owner identity/memory/skill text saves apply to subsequent runtime requests. Explicit sample mode retains browser-only drafts. Development uses environment credentials; production web credential enrollment remains planned. Settings do not grant execution permissions or change policy. See [the implementation scope](docs/ui-workspace.md).

# Phase 17 — Focus Threads UI

## Objective

Implement Muse-style main chat + dedicated side threads.

## Work

- thread list
- thread creation
- auto-created Focus Threads
- compact summaries
- thread files
- thread tasks
- thread artifacts
- thread activity
- main-chat reference cards

## Exit Criteria

```text
✓ main chat remains clean
✓ deep work moves to thread
✓ relevant thread summary can appear in main chat
✓ full thread history remains available
```

---

# Phase 18 — Rich Card System

## Objective

Add structured UI without letting agents generate arbitrary frontend code.

## First Cards

- project update
- recommendation
- task
- approval
- TOTP
- bounded write-session status
- subagent progress
- cross-agent request
- security event
- file/artifact
- video filmstrip
- generic info

## Contract

Agent returns schema.

Frontend renders component.

## Exit Criteria

```text
✓ card schemas versioned
✓ bad schema fails safely
✓ normal chat still works without cards
✓ actions map to supported app operations
```

---

# Phase 19 — Activity and Approvals

## Objective

Provide visibility without cluttering main chat.

## Activity

Show:

- active tasks
- running subagents
- heartbeat investigations
- recent tool calls
- recent failures

## Approvals

Show:

- TOTP writes
- distinguish one-action authorization from bounded-session authorization
- session manifest/scope, expiry countdown, remaining budgets, and End session control
- per-action outcomes, including uncertain results awaiting reconciliation
- Guard exception requests
- cross-agent requests
- sensitive tool requests
- maintenance recommendations

## Exit Criteria

```text
✓ all pending approvals visible
✓ active workers visible
✓ main chat stays clean
✓ session status comes from the backend; UI timers do not authorize execution
✓ User can end a session; UI explains that already dispatched writes are not undone
```

---

# Phase 20 — Memory Management UI

## Objective

Give User full control over remembered information.

## Work

- search
- filter by agent
- filter by source
- filter by confidence
- edit
- delete
- verify
- mark stale
- show provenance
- show sensitivity
- show shared/private state

## Exit Criteria

```text
✓ every memory can be inspected
✓ source visible
✓ confidence visible
✓ edits audited
✓ deletion audited
```

---

# Phase 21 — Maintenance Sessions

## Objective

Formalize human-approved cleanup.

## Work

- maintenance recommendation generator
- completed thread review
- stale task review
- low-confidence memory review
- artifact retention review
- duplicate thread suggestions
- pin/archive recommendations

## Rules

Agent recommends.

User decides.

## Exit Criteria

```text
✓ agent can prepare maintenance agenda
✓ no silent durable workspace archival
✓ accepted changes are audited
```

---

# Phase 22 — Packaging and Desktop App

## Objective

Make the system feel like a real desktop assistant.

## Work

- Tauri wrapper
- system tray
- notifications
- deep links
- file open/share hooks
- secure local auth/session
- reconnect behavior
- launch-on-login option

## Exit Criteria

```text
✓ Mac desktop package
✓ connects securely to LXC
✓ notifications work
✓ same core web app remains usable in browser
```

---

# Phase 23 — Mobile-Friendly Companion

## Objective

Make the responsive UI genuinely useful on phone.

Not a full native rewrite.

## Focus

- main chat
- quick reply
- approvals
- TOTP
- notifications
- Focus Thread summaries
- task status
- image/file sharing

## Exit Criteria

```text
✓ usable from phone browser/PWA
✓ approval flow easy on mobile
✓ no desktop-only critical action
```

---

# Phase 24 — Reliability / Hardening

## Objective

Make the platform trustworthy enough for everyday use.

## Work

- crash recovery
- task resume
- idempotency
- duplicate-event handling
- DB backups
- migration framework
- log rotation
- storage quotas
- sandbox cleanup
- stale worker cleanup
- rate limits
- secret rotation
- timeout standards
- retry standards
- health endpoints
- monitoring

## Exit Criteria

```text
✓ daemon survives restart
✓ tasks recover safely
✓ DB backup/restore tested
✓ failed workers cleaned
✓ no orphan approval state
```

---

# Phase 25 — Voice Preparation

## Objective

Prepare for voice without changing the runtime architecture.

## Work

- modality field in message/event schema
- audio attachment support
- streaming event hooks
- interruption/cancel concepts
- response chunking suitable for TTS

No full voice feature yet.

---

# Later — Voice Conversations

## Components

```text
microphone
→ speech-to-text
→ Janet/Harley
→ text response
→ text-to-speech
```

Future features:

- natural interruption
- low-latency streaming
- voice identity
- headset/Bluetooth
- car use
- lock-screen interaction

---

# Later — Google Assistant Replacement Direction

Long-term capabilities may include:

- wake word
- timers
- reminders
- calls/messages through supported integrations
- calendar
- navigation hooks
- home automation
- media control
- phone-level shortcuts
- context-aware voice interaction
- proactive spoken notifications

This is not a v1 requirement.

---

# Milestone Grouping

## Milestone A — Multi-Agent Core

Phases:

```text
0–4
```

Outcome:

- Janet
- Harley
- tasks
- Focus Threads
- temporary subagents
- archives

---

## Milestone B — Safe Operator

Phases:

```text
5–10
```

Outcome:

- model routing
- root policy
- Guard
- Opus escalation
- secret broker
- sandbox shell
- media/file workflows

---

## Milestone C — Connected Operator

Phases:

```text
11–15
```

Outcome:

- n8n
- TOTP writes
- heartbeats
- long-term memory
- Janet/Harley approval-gated collaboration

---

## Milestone D — Daily-Driver App

Phases:

```text
16–23
```

Outcome:

- chat-first desktop UI
- Focus Threads
- rich cards
- approvals
- memory control
- maintenance
- desktop package
- phone-friendly companion

---

## Milestone E — Hardened Platform

Phase:

```text
24+
```

Outcome:

- recovery
- backups
- stable upgrades
- voice readiness
- future assistant replacement path

---

# Recommended Immediate Next Work

Do not start with the UI.

The next implementation sequence should be:

```text
1. Fork and baseline TinyClaw
2. Inventory existing architecture
3. Add Janet/Harley agent registry
4. Add separate state/workspaces
5. Add tasks
6. Add temporary subagent lifecycle
7. Add audit foundation
8. Add security/root-policy foundation
```

The app becomes valuable only after the runtime can reliably represent:

- who is acting
- what task is running
- what worker exists
- what was allowed
- what was blocked
- what needs approval
- what should be remembered

---

# Branch Strategy

Use:

```text
main
```

as the always-deployable branch.

Feature branches:

```text
feat/multi-agent-runtime
feat/tasks
feat/subagents
feat/model-router
feat/security-policy
feat/guard
feat/secrets
feat/sandbox
feat/n8n
feat/heartbeat
feat/memory
feat/app-shell
...
```

Merge into `main` only when:

- tests pass
- migration is documented
- deployment remains reproducible
- feature has acceptance criteria

---

# Deployment Goal

Eventually User should be able to update the LXC with one command.

Example desired behavior:

```text
tinyclaw-update
```

which performs:

```text
git fetch/pull main
→ install/update dependencies
→ run migrations
→ build
→ run tests/smoke check
→ restart service
→ verify health
```

Do not implement unsafe blind auto-updates.

The update command should fail safely before replacing a healthy running service if build or migration checks fail.

---

# Definition of v1

v1 is complete when all of the following are true:

- Janet and Harley are independent permanent agents
- each has a main chat
- Focus Threads work
- both can proactively investigate
- both can spawn temporary subagents
- subagents have archives
- memory is separate and source-aware
- shell runs in controlled sandboxes
- agents can install ordinary tools
- secrets are brokered
- Guard can block actions
- blocked actions can be reviewed by Opus
- root-of-trust actions are never UI-approvable
- external writes go through n8n/TOTP
- cross-boundary Janet/Harley communication requires User
- desktop-first chat UI works
- phone layout is usable
- text/files/images work
- rich cards work
- audit trail is inspectable
- maintenance recommendations exist
- system survives normal restart/recovery

Voice is explicitly post-v1.

---

# Product Test

Before accepting a feature, ask:

> Does this make Janet or Harley more useful while keeping User in control?

If the answer is no, it probably does not belong in the core product.


## October 19 thread and recovery delivery

Step 4 is complete locally: authenticated thread file upload/download, schema-v9 durable human decisions with idempotent retries, and structured worker results/archive downloads. Local validation: 2,445 Rust tests and 14 frontend checks passed; 43 browser checks passed with one intentional skip; five existing host sandbox Rust tests remain ignored. Both gateway modes passed complete stopped fixture-state backup/restore. The Linux service unit newline bug is fixed, and concurrent write-budget admission is tested across independent connections.

Step 5 is deferred by User; target VM, live Guard/n8n/TOTP and always-on acceptance remain open. Production credential enrollment remains unfinished. Final-head CI is required; At that historical checkpoint PR #1 was draft and main was unchanged; the later authorized consolidation superseded that restriction. See [deployment acceptance](docs/deployment-acceptance.md).
