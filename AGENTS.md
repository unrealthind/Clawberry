# AGENTS.md

## Mandatory New-Session GitHub Check

At the start of every new session, before reporting project progress, planning implementation, or changing code:

1. Check the current branch and working tree with `git status --short --branch`.
2. Fetch the latest GitHub state with `git fetch origin`. Local history alone is not evidence of the current project state.
3. Compare `HEAD` with its upstream branch. If its remote-tracking reference is missing, fetch that branch explicitly before comparing. If no upstream is configured, check the matching branch on `origin` and the remote default branch. Inspect incoming commits and relevant changes to code, phase trackers, and work logs.
4. Safely integrate applicable upstream changes before implementation. Preserve uncommitted work and other contributors' commits; never reset, discard, or force-push to make branches match. Resolve overlapping changes and run the relevant checks before delivery.
5. Base progress reports and next steps on the checked remote and local state. Clearly distinguish local-only work from work already on GitHub.

If fetching fails because of network access, authentication, or permissions, say so explicitly. Do not claim the checkout or progress report is up to date; continue only with work that can safely proceed from the known local state.

## Project

This repository is a fork of TinyClaw being evolved into a lightweight, self-hosted multi-agent assistant platform.

The product goal is a **baby of TinyClaw + Muse**:

- TinyClaw-style lightweight runtime and channel/tool architecture
- Muse-style persistent personal assistants, proactive behavior, skills, memory, and rich interaction
- Strong user control, auditability, security boundaries, and explicit approval for sensitive actions
- Desktop-first command center with a clean chat-first experience
- LXC-hosted runtime designed to stay online continuously

The system is intended to become User's primary personal/work assistant platform over time.

---

# Core Philosophy

The agents are **extensions of User's hands, not replacements for User's judgment**.

Agents may:

- monitor
- investigate
- research
- plan
- delegate
- spawn temporary subagents
- use tools
- run shell commands
- install software inside approved sandboxes
- prepare actions
- make recommendations

Agents must not silently take ownership of decisions that materially affect User, external systems, finances, security, or important commitments.

User remains in control.

---

# Primary Permanent Agents

There are two primary permanent agents.

## Janet

Janet is the **work/business agent**.

Primary responsibilities:

- business operations
- project monitoring
- deadlines
- client-related work
- company communications
- business research
- project status
- finance-related visibility
- permit/application workflows
- business task management
- n8n-triggered business workflows
- proactive business monitoring
- recommendations and escalation

Janet is expected to stay ahead of work.

Janet may proactively:

- create tasks
- investigate issues
- spawn subagents
- monitor deadlines
- inspect read-only sources
- prepare recommendations
- surface risks
- prepare draft actions

Janet must not perform external write actions outside the approved write path.

---

## Harley

Harley is the **personal agent**.

Primary responsibilities:

- personal planning
- calendar awareness
- travel
- household tasks
- media
- personal research
- reminders
- errands
- personal projects
- proactive personal monitoring
- recommendations and escalation

Harley may proactively:

- create tasks
- investigate issues
- spawn subagents
- monitor personal commitments
- prepare plans
- research options
- surface risks or forgotten commitments

Harley must not silently cross into Janet's business domain.

---

# Janet ↔ Harley Boundary

Janet and Harley are peers.

Neither agent has authority over the other.

They must have:

- separate identity
- separate memory
- separate workspace
- separate permissions
- separate tool access rules
- separate secret scopes
- separate heartbeat/watch loops
- separate task history

## Cross-boundary communication

Cross-boundary communication requires explicit approval from User.

Default:

```text
Janet ──X── Harley
```

If Janet wants information from Harley, or Harley wants information from Janet:

1. Create a cross-boundary request.
2. Explain exactly what information is needed.
3. Ask User for approval.
4. If approved, open a narrowly scoped message exchange.
5. Return only the information needed.
6. Close the temporary communication scope.

No direct database access is permitted across the Janet/Harley boundary.

Janet must never directly read `harley.db`.

Harley must never directly read `janet.db`.

Cross-boundary approval does not grant general memory access.

---

# Runtime Topology

Initial deployment target:

```text
One LXC
│
├── TinyClaw Supervisor
├── Agent Runtime
├── Janet
├── Harley
├── Message Bus
├── Model Router
├── Policy Engine
├── Llama Guard
├── Approval Service
├── Secret Broker
├── Tool Broker
├── n8n Bridge
├── Memory Services
├── Task Archive
└── Agent Sandboxes
```

Do not create a separate LXC for every permanent agent in v1.

Janet and Harley should be logically isolated while sharing the same primary TinyClaw LXC.

High-risk execution workloads may later run inside:

- disposable containers
- dedicated sandboxes
- isolated worker LXCs

Do not couple agent identity to deployment location.

---

# Permanent Agents vs Subagents

Permanent agents:

- Janet
- Harley

Subagents are normally **temporary**.

Examples:

- researcher
- coder
- finance analyst
- email investigator
- video analyst
- document reviewer
- web researcher
- travel researcher

## Subagent lifecycle

```text
Parent Agent
    ↓
Create Task
    ↓
Spawn Temporary Subagent
    ↓
Grant Scoped Context + Tools + Permissions
    ↓
Execute Task
    ↓
Return Structured Result
    ↓
Archive Logs / Artifacts / Sources
    ↓
Promote Useful Memory if Appropriate
    ↓
Destroy Active Subagent
```

Subagents should normally disappear after the task is finished.

Their history remains available to the parent agent.

---

# Subagent Archive

Completed subagents must retain an auditable task archive.

Suggested structure:

```text
archive/
└── task_<id>/
    ├── summary.md
    ├── transcript.jsonl
    ├── tool_calls.jsonl
    ├── shell.log
    ├── artifacts/
    ├── sources.json
    ├── costs.json
    └── memory_candidates.json
```

The archived subagent is not considered an active agent.

Parent agents should be able to:

- list archived subagents
- inspect task results
- inspect tool usage
- inspect artifacts
- inspect source provenance
- review cost
- retry a task
- promote useful knowledge
- reject memory candidates

Logs are not memory.

---

# Memory Architecture

Use separate memory domains.

Recommended initial databases:

```text
data/
├── system.db
├── janet.db
├── harley.db
└── shared.db
```

## `system.db`

Infrastructure and runtime state.

Suggested entities:

- agents
- sessions
- rooms
- messages
- tasks
- task_dependencies
- subagents
- delegations
- tool_calls
- approvals
- guard_decisions
- opus_reviews
- audit_events
- model_usage
- secret_references
- heartbeat_runs
- notifications

## `janet.db`

Janet-owned long-term memory.

## `harley.db`

Harley-owned long-term memory.

## `shared.db`

Only explicitly shared/promoted information.

No permanent agent should automatically write private memories into shared memory.

---

# Long-Term Memory Rules

Janet and Harley may manage long-term memory automatically.

Each memory should include:

- memory ID
- owner agent
- content
- category
- confidence
- provenance/source type
- source reference
- created timestamp
- last verified timestamp
- sensitivity classification
- visibility
- optional expiry/review date

Example:

```text
memory_id
agent_owner
content
category
confidence
source_type
source_ref
created_at
last_verified_at
sensitivity
visibility
```

## Requirements

- confidence must be stored
- provenance/source must be stored
- User must be able to inspect memory
- User must be able to edit memory
- User must be able to delete memory
- sensitive categories are excluded from automatic memory by default
- task logs remain separate from semantic memory
- subagent output must not be dumped wholesale into long-term memory

Useful facts should be selectively promoted.

---

# Heartbeat / Watch Loops

Janet and Harley each have their own continuous heartbeat/watch loop.

These loops should:

- wake on a configurable schedule
- inspect relevant state
- identify stale items
- identify missed commitments
- identify deadlines
- detect failed workflows
- detect risks
- inspect relevant read-only data
- trigger investigations
- spawn temporary subagents when useful
- prepare recommendations

## Heartbeat behavior

When a heartbeat notices something important:

1. Investigate autonomously.
2. Gather evidence.
3. Spawn subagents if useful.
4. Build a concise recommendation.
5. Notify User.
6. Wait for further authorization before taking write action.

The heartbeat itself should not silently perform external writes.

---

# Proactive Behavior

Janet and Harley may proactively:

- create tasks
- create internal investigations
- spawn temporary workers
- monitor existing work
- inspect read-only sources
- prepare drafts
- gather evidence
- build recommendations
- notify User of risks and opportunities

This is expected behavior.

They should not wait for User to explicitly ask about every issue.

---

# n8n Role

Most operational integrations and external actions should run through n8n.

TinyClaw is the agent/orchestration layer.

n8n is the workflow/action layer.

Typical pattern:

```text
Janet / Harley
      ↓
Tool request
      ↓
TinyClaw Tool Broker
      ↓
n8n
      ↓
External system
```

Examples:

- email
- pricing
- Invoice Ninja
- calendar
- project systems
- file systems
- permit systems
- external APIs

---

# Write Actions and TOTP

All external write actions are behind TOTP.

This is a hard product requirement.

Examples:

- sending email
- replying to email
- updating records
- creating invoices
- modifying quotes
- changing calendar entries
- financial actions
- project writes
- external data changes

Read-only investigation may be autonomous.

Write execution requires the approved n8n/TOTP path.

Do not introduce alternative write paths that bypass TOTP.

Support two explicit authorization modes:

- **Single action:** User reviews the exact operation, target, and payload. TOTP authorizes one execution; changing the reviewed payload requires new authorization.
- **Bounded session:** User reviews a fixed capability manifest. One fresh TOTP opens a short-lived session for multiple operations within its approved scope and per-operation budgets. A new TOTP is not required for each operation inside that session.

Use single-action approval when one operation is needed. A session is not blanket authority: bind it to the requesting agent, task, authenticated broker identity, resource scope, manifest hash, expiry, and operation budgets. Enforce these constraints server-side on every action. A human-readable purpose explains intent but does not enforce resource scope.

The existing email-management n8n gateway is the integration reference: request manifest → review → authorize with TOTP → execute allowlisted actions → end session. Its current session lasts at most five minutes and supports folder organization, message move/copy/archive, read state, flags, and tags. It does not support sending email, deletion, arbitrary IMAP commands, or caller-controlled connection settings. Other write workflows require their own reviewed contracts.

The Tool/Secret Broker holds webhook credentials and write-session tokens. User-entered TOTP travels through a dedicated authenticated approval flow directly to the gateway; codes and bearer tokens must not enter model context, conversation history, logs, analytics, or artifacts. Models receive safe session references and status only. Subagents do not inherit the session token or grant automatically.

Session grants do not bypass deterministic policy, Guard, root-of-trust denials, or Janet/Harley boundaries. Budget reservation, authorization consumption, and replay protection must be atomic. Expired or revoked sessions cannot admit new actions; revocation does not undo an already dispatched write. End sessions when work finishes and permit User to end them early.

Retain durable correlation between approval, manifest, session, agent, task, action, n8n execution, and result. Reconcile uncertain outcomes before retrying; a failed verification or timeout may follow a successful external write. Do not treat a gateway's `retry_allowed` flag alone as evidence that replay is safe.

---

# Shell Access

Janet and Harley should have shell capability.

They need to be able to perform real work.

Examples:

- run Python
- inspect files
- manipulate media
- use Git
- use ffmpeg
- use yt-dlp
- use ImageMagick
- use Pandoc
- use Playwright
- install task-specific utilities
- create scripts
- process documents

Example task:

```text
Review a video
→ install/check yt-dlp
→ download video
→ install/check ffmpeg
→ extract frames
→ create contact sheets / filmstrip
→ inspect frames
→ optionally inspect audio/transcript
→ return findings
```

---

# Shell Isolation

Do not give agents unrestricted permanent root on the main TinyClaw LXC.

Preferred model:

```text
Agent
  ↓
Agent Sandbox
  ├── shell
  ├── filesystem
  ├── network
  ├── installed packages
  └── workspace
```

An agent may have root-like capability inside its sandbox without receiving unrestricted root on the orchestration LXC.

Agent sandboxes should be independently permissioned.

---

# Software Installation

Agents may install software needed to complete tasks inside approved execution environments.

Examples that should normally be allowed:

- ffmpeg
- yt-dlp
- jq
- ripgrep
- ImageMagick
- Pandoc
- Python packages
- Node packages
- document/media utilities

Prefer first-class runtime operations:

```text
package.search
package.install
package.remove
package.list
```

over agents manually using arbitrary privileged installation paths.

Package actions must be logged.

High-risk installation actions should be escalated.

Examples:

- adding unknown third-party repositories
- enabling system daemons
- installing remote shell servers
- adding VPN/network-level software
- kernel-level changes
- host-level container runtimes
- privileged mounts

---

# Skills

Reusable successful workflows may become skills.

Example:

```text
skills/video-review/
├── SKILL.md
├── extract_frames.sh
├── contact_sheet.py
└── config.yaml
```

Agents may propose skills.

Skill creation should be auditable.

Skills must not silently expand privileges.

---

# Model Routing

Model selection is controlled by the runtime.

Agents should request an effort/profile, not hard-code provider model names into their prompts.

Preferred routing:

## Fast

Current target:

- Claude Haiku tier

Use for:

- extraction
- classification
- metadata handling
- short summaries
- lightweight transformations
- routine subagent work
- inexpensive preprocessing

## Normal

Current target:

- Claude Sonnet 5.5
- approximately 70% of normal workload

Use for:

- Janet
- Harley
- most conversations
- tool use
- research
- normal coding
- project analysis
- shell orchestration
- most subagents

## Deep

Current target:

- Claude Opus 5.5

Use for:

- difficult planning
- architecture
- complicated debugging
- high-impact decisions
- conflicting evidence
- repeated task failure
- security review
- Guard-block review
- difficult multi-agent planning

Suggested interface:

```text
reasoning.escalate(
    profile="deep",
    reason="..."
)
```

Do not make Janet or Harley depend on a specific model string.

---

# Guard Model

Use:

```text
meta-llama/Llama-Guard-4-12B
```

via Hugging Face Inference Providers API.

Llama Guard is part of the execution security chain.

It is not the primary reasoning model.

---

# Security Execution Flow

Normal tool/shell execution should follow:

```text
Agent wants action
      ↓
Deterministic Policy Checks
      ↓
Llama Guard
      ↓
SAFE → Execute
BLOCK → Escalate to Opus
```

If Guard blocks:

```text
Guard BLOCK
    ↓
Claude Opus 5.5 review
    ↓
├── agree unsafe → deny
├── safer alternative → reformulate and re-run through Guard
└── justified exception → request User approval
```

Important:

**Opus cannot silently override Guard.**

Opus is a reviewer.

User is the authority for approvable exceptions.

---

# Scoped Approvals

Approvals should be narrow and temporary.

Do not grant blanket authority when a single operation is needed.

Example approval:

```text
approval_id: appr_9182
agent: janet
task: TASK-551
operation: service.restart
target: nginx
expires: after operation
reuse: false
```

Approval should be scoped to:

- agent
- task
- operation
- resource/target
- duration
- number of executions

For bounded write sessions, execution count means the reviewed per-operation budgets, not single-use authorization of every action. Manifest or scope changes require fresh approval/TOTP. The UI must distinguish single-action approval from session authorization and show the active session's expiry, remaining budgets, results, and an End session control. Neither mode grants cross-agent communication approval.

---

# Root of Trust

There is a tiny category of actions that can never be approved through the agent UI.

These are absolute deny operations.

Examples:

- disabling Guard
- bypassing the policy engine
- modifying root-of-trust rules
- dumping/exporting raw secrets
- deleting/tampering with the security DB
- deleting/tampering with the audit trail
- granting permanent unrestricted host root
- modifying approval logic to self-authorize
- allowing an agent to permanently disable security controls
- directly accessing another permanent agent's private DB outside approved interfaces

If an agent requests one of these:

```text
HARD DENY

This action cannot be approved through the agent system.

Manual host administration by User is required.
```

Neither Guard review, Opus, Janet, Harley, nor UI approval may override this.

Root-of-trust configuration should be owned by the host administrator and non-writable by the TinyClaw runtime.

Suggested location:

```text
/etc/tinyclaw/security/root-policy.toml
```

---

# Secret Broker

Raw secrets must not be exposed to models.

Agents should reference named credentials.

Example:

```text
credential: pricing_api
```

not:

```text
Authorization: Bearer <raw-secret>
```

Secret flow:

```text
Agent
  ↓
Tool Broker
  ↓
Permission Check
  ↓
Secret Broker
  ↓
Inject credential
  ↓
Perform request
  ↓
Redact sensitive output
  ↓
Return safe result to agent
```

The agent may know:

- a credential exists
- what capability it enables
- whether it is allowed to use it

The agent must not receive raw secret material.

---

# Secret Scope

Secrets are scoped by agent and capability.

Example:

```yaml
invoice_ninja_read:
  allowed_agents:
    - janet

pricing_api:
  allowed_agents:
    - janet

personal_calendar:
  allowed_agents:
    - harley

shared_calendar:
  allowed_agents:
    - janet
    - harley
```

Subagents do not automatically inherit all secrets from the parent.

Parent agents grant specific capabilities to each worker.

---

# Tool Permissions

Subagents must be least-privilege.

Example:

```text
Video Analysis Worker

shell: yes
network: yes
filesystem: scoped
package install: yes
secrets: no
Harley memory: selected context only
Janet access: no
```

A parent agent must never accidentally grant its entire privilege set to a temporary worker.

---

# Inter-Agent Communication

Communication must be first-class and auditable.

Suggested primitives:

```text
agent.send
agent.ask
agent.delegate
agent.broadcast
agent.spawn
agent.wait
agent.status
```

Every inter-agent message should be persisted.

Example:

```json
{
  "from": "janet",
  "to": "research_184",
  "type": "delegate",
  "task_id": "task_123",
  "message": "Investigate the permit status."
}
```

Permanent-agent cross-boundary communication remains approval-gated.

Subagents may communicate within the scope allowed by their parent/task.

---

# Tasks

Tasks are first-class objects.

Suggested fields:

- task ID
- owner agent
- created by
- title
- description
- priority
- status
- due date
- source
- related focus thread
- subagents
- dependencies
- evidence
- recommendation
- required approvals
- artifacts
- cost
- timestamps

Suggested statuses:

```text
queued
investigating
waiting
blocked
needs_manni
approved
executing
completed
cancelled
```

---

# Maintenance Sessions

Janet and Harley must not automatically archive/close maintained side chats, important workspaces, or long-lived organizational structures.

Instead, they should recommend maintenance.

Examples:

- archive completed focus threads
- merge duplicate threads
- pin important threads
- clean stale tasks
- review low-confidence memories
- remove outdated memories
- compress/archive old subagent artifacts

Maintenance should happen with User.

Agents should periodically recommend a maintenance session.

Example:

```text
We have not done maintenance in 12 days.
There are 19 completed threads and 7 low-confidence memories.
I recommend a cleanup review.
```

User approves lifecycle changes.

---

# Conversation Model

Each permanent agent has one primary main chat.

## Janet

- one primary work chat

## Harley

- one primary personal chat

The main chat is intended to be used most of the time.

---

# Focus Threads

Dedicated side chats/workspaces are called **Focus Threads**.

Use them for:

- projects
- investigations
- trips
- media reviews
- focused research
- complex tasks
- long-running work

A Focus Thread may contain:

- conversation
- tasks
- subagents
- artifacts
- evidence
- approvals
- decisions
- files
- history

The main chat may show compact summaries of Focus Threads when relevant.

Do not flood the main chat with entire side-thread history.

---

# Main Chat Context

The main chat should stay efficient.

Preferred context:

```text
recent conversation
+ active working state
+ current priorities
+ important commitments
+ relevant long-term memories
+ compact summaries of relevant Focus Threads
```

Do not automatically inject all historical conversations into every model call.

Historical data remains retrievable.

---

# UI Philosophy

The primary UI is a **clean chat app with occasional rich cards**.

Do not make the default Janet/Harley experience a dashboard.

The app should feel conversation-first.

Default:

```text
chat
chat
chat
rich card when useful
chat
focus thread link
chat
```

Avoid permanent walls of:

- charts
- model usage
- agent status
- security panels
- task grids
- operational dashboards

Those belong in secondary screens.

---

# Rich Cards

Agents do not generate React/UI code dynamically for normal cards.

Use predefined reusable card schemas.

Suggested cards:

- project update
- approval
- TOTP
- task
- recommendation
- subagent progress
- cross-agent request
- security event
- file/artifact
- video/filmstrip
- memory
- generic structured info

Agents return structured card payloads.

Example:

```json
{
  "type": "recommendation_card",
  "title": "Example Project Permit",
  "status": "Needs review",
  "summary": "City response received. Two items need your decision.",
  "actions": [
    "open_thread",
    "review_evidence"
  ]
}
```

Frontend renders the component.

Target usage:

- ~80% predefined cards
- ~15% generic schema-driven cards
- ~5% intentionally added custom components

Do not let agents write arbitrary frontend code to render one-off cards in normal operation.

---

# Frontend Direction

Desktop-first command center.

Must also work well on phone.

Do not build the complete UI stack from scratch.

Preferred building blocks:

- React / Next.js
- shadcn/ui
- assistant-ui
- assistant-ui tool UI
- optional Refine for CRUD-heavy management screens
- Tauri later for desktop packaging
- responsive web/PWA foundation

The primary app experience is chat-first.

Suggested minimal navigation:

```text
Janet
Harley
────────
Threads
Tasks
Activity
Approvals
More
```

Secondary areas may include:

- Memory
- Security
- n8n
- Tools
- Settings
- Audit
- Costs

---

# Global App Settings and Agent Settings

Keep these scopes separate, without duplicate navigation entries:

- Global app settings open from the User profile footer or the standard settings keyboard shortcut. They contain workspace appearance, notifications, calendar preferences, gateway connection, shared Claude API configuration, and shared model profiles.
- Each permanent agent has one dedicated settings/setup page accessed beside that agent. Consolidate identity/personality, memory source files, skills, credential references and access boundaries there. Do not put agent setup in global settings or scatter it across standalone Skills/Vault workspace pages.
- Project/conversation Files is separate from agent identity configuration. Do not create a second personality form alongside the identity source editor.
- Settings previews and model drafts do not activate runtime capabilities, change policy, accept raw credentials, or grant cross-agent access. Secure backend integration is required before live credential enrollment or runtime model-setting changes.

---

# v1 Modalities

v1 supports:

- text
- files
- images

v1 does not require voice.

However, the conversation/event architecture must be designed so voice can be added later without redesigning the agent runtime.

Future:

```text
Speech-to-Text
      ↓
existing Janet/Harley runtime
      ↓
response
      ↓
Text-to-Speech
```

Long-term product direction includes:

- voice conversation
- hands-free use
- wake word
- phone integration
- Bluetooth/car mode
- home automation
- potentially replacing Google Assistant for User's everyday assistant interactions

Do not build v1 in a way that blocks this path.

---

# Model Usage Tracking

Track model usage by:

- agent
- subagent
- task
- model profile
- provider
- input tokens
- output tokens
- cost
- timestamp

Support budgets.

Examples:

```text
Janet daily budget
Harley daily budget
subagent task budget
deep-reasoning budget
```

Budget limits must not silently downgrade critical reasoning without surfacing that decision.

---

# Auditability

Every important action should be reconstructable.

Audit:

- agent messages
- subagent creation
- inter-agent communication
- shell calls
- package installs
- tool calls
- secret references
- Guard decisions
- Opus reviews
- approvals
- TOTP-triggered writes
- n8n workflow execution
- memory promotion
- maintenance changes
- model usage

The system should support questions like:

```text
Why did Janet recommend this?
What did the research agent inspect?
Why was this command blocked?
Who approved this write?
What did this task cost?
What memory came from this investigation?
```

---

# Failure Handling

Agents must report meaningful failures.

If a task fails:

1. record failure
2. retain logs
3. diagnose
4. retry safely if appropriate
5. use a safer alternate method if possible
6. escalate to Opus for difficult failures
7. notify User when intervention is required

Do not silently swallow errors.

---

# Source / Confidence Discipline

When agents make claims from:

- emails
- web research
- APIs
- project data
- subagents
- files

they should retain provenance.

Recommendations should distinguish:

- fact
- inference
- assumption
- recommendation

Confidence should be explicit when uncertainty matters.

---

# Design Rules

## Do

- keep the runtime modular
- keep agent identity separate from model provider
- keep tool execution separate from reasoning
- keep secrets outside model context
- keep writes behind TOTP
- keep cross-boundary Janet/Harley communication approval-gated
- keep subagents ephemeral by default
- keep audit history
- keep main chat compact
- use Focus Threads for dedicated work
- let permanent agents proactively investigate
- let agents install tools inside sandboxes
- use Guard before sensitive execution
- escalate blocked actions to Opus
- ask User for scoped approval when necessary
- preserve a hard root-of-trust deny layer

## Do Not

- create a separate LXC for every bot by default
- make Janet subordinate to Harley
- make Harley subordinate to Janet
- give models raw secrets
- allow unrestricted host root
- allow agents to bypass Guard
- let Opus silently override Guard
- bypass n8n/TOTP for writes
- allow direct Janet↔Harley DB access
- keep every subagent alive forever
- dump task logs into semantic memory
- auto-archive important threads without User
- build the default UI as a huge dashboard
- let agents dynamically code arbitrary UI for every card
- hard-code business logic into model prompts when it belongs in policy/runtime code

---

# Suggested Repository Direction

Possible structure:

```text
/
├── crates/
│   ├── runtime/
│   ├── agents/
│   ├── memory/
│   ├── message-bus/
│   ├── tasks/
│   ├── models/
│   ├── guard/
│   ├── policy/
│   ├── approvals/
│   ├── secrets/
│   ├── tools/
│   ├── sandbox/
│   ├── n8n/
│   ├── audit/
│   └── gateway/
│
├── app/
│   ├── components/
│   ├── assistant/
│   ├── cards/
│   ├── threads/
│   ├── tasks/
│   ├── approvals/
│   ├── activity/
│   ├── memory/
│   └── settings/
│
├── agents/
│   ├── janet/
│   │   ├── AGENT.md
│   │   └── config.toml
│   └── harley/
│       ├── AGENT.md
│       └── config.toml
│
├── skills/
├── config/
├── docs/
└── AGENTS.md
```

This structure is directional, not mandatory.

Reuse TinyClaw's existing abstractions where they fit.

Avoid unnecessary rewrites.

---

# Implementation Priorities

## Phase 0 — Fork Baseline

- fork TinyClaw
- build unchanged
- run in target LXC
- document existing architecture
- document existing database
- document existing tool/channel/provider abstractions
- create baseline tests
- preserve upstream compatibility where practical

## Phase 1 — Multi-Agent Foundation

- permanent agent registry
- Janet
- Harley
- isolated sessions
- agent-owned workspaces
- separate memory namespaces/DBs
- agent message bus
- task model
- basic Focus Threads

## Phase 2 — Subagents

- ephemeral worker creation
- scoped context
- scoped tools
- lifecycle management
- task archive
- structured result handoff
- memory candidate promotion

## Phase 3 — Security + Secrets

- deterministic policy engine
- Llama Guard integration
- Opus escalation
- approval model
- immutable deny/root-of-trust layer
- secret broker
- credential injection
- audit events
- redaction

## Phase 4 — Sandboxed Execution

- shell tool
- agent sandboxes
- package manager operations
- persistent per-agent tools
- task workspaces
- artifact handling
- network policy
- high-risk command approval paths

## Phase 5 — n8n Integration

- read actions
- workflow invocation
- TOTP write handoff
- single-action and bounded-session authorization
- broker-held session credentials, expiry/revocation, and atomic budgets
- execution status
- error handling
- uncertain-write reconciliation before retry
- audit correlation
- heartbeat checks for failed workflows

## Phase 6 — Heartbeats + Proactivity

- Janet watch loop
- Harley watch loop
- proactive tasks
- recommendation pipeline
- notifications
- investigation-before-notification behavior

## Phase 7 — Memory

- automatic long-term memory
- confidence
- provenance
- sensitive-category exclusions
- inspect/edit/delete
- verification
- task-log separation
- shared-memory rules

## Phase 8 — App

- desktop-first responsive UI
- Janet main chat
- Harley main chat
- Focus Threads
- files/images
- reusable rich cards
- tasks
- approvals
- activity
- memory management
- security/audit screens
- mobile-friendly layout

## Phase 9 — Maintenance + Quality

- maintenance sessions
- archive recommendations
- memory review
- task cleanup
- artifact compression policies
- cost dashboards
- reliability
- tests
- migration tooling

## Later

- voice
- TTS/STT
- wake word
- native/mobile integrations
- home automation
- richer OS-level assistant capabilities
- deeper Google Assistant replacement features

---

# North Star

The system should feel like this:

```text
User
│
├── Janet — always-on work agent
│     └── temporary workers as needed
│
└── Harley — always-on personal agent
      └── temporary workers as needed
```

Janet and Harley:

- remember
- monitor
- investigate
- plan
- delegate
- recommend
- prepare
- stay ahead

But User remains the final authority.

The goal is not to build autonomous executives that replace User.

The goal is to build dependable, proactive, security-conscious digital extensions of User's hands.

## Local UI development access

User authorized token-free direct-loopback development via `gateway --local-ui` or `daemon --local-ui` (`gateway.local_ui` is otherwise false). Refuse non-loopback binding, tunnels and trusted proxy headers in this mode; reject nonlocal Host/Origin, cross-site fetches and forwarding headers. Do not expose local mode through a reverse proxy. Remote deployments retain pairing. This changes human UI access only; it grants no bypass of policy, Guard, root-of-trust, TOTP writes, secret isolation or Janet/Harley boundaries.

## Current product delivery — October 19

Follow [docs/product-plan.md](docs/product-plan.md): local real Claude routing, default connected workspace and backend settings first, then complete thread artifacts/decisions/results, then deployed approval/always-on acceptance. Development provider credentials come from host environment or git-ignored `.env.local`; production web enrollment remains planned. Live settings use dedicated human-only endpoints with closed nonsecret schemas and revision checks. Agent guidance/skill text is owner-scoped and cannot grant execution privileges. Schema v9 adds durable human thread decisions alongside v8 persistent UI configuration; preserve matching stopped-state backups for upgrades/rollback. Explicit `?mode=sample` retains previews; never substitute sample data on live failures.


## Current steps 4–5 acceptance

Step 4 now connects authenticated file transfer, human-only idempotent thread decisions and structured worker archives in the live workspace. Step 5 remains gated on actual target-host and live Guard/n8n/TOTP evidence; local fixture tests and a generated unit are not deployment acceptance. Follow [docs/deployment-acceptance.md](docs/deployment-acceptance.md). Keep production enrollment and external writes disabled until their dedicated contracts/gates pass. PR remains draft; main is unchanged.

User has deferred step 5 deployment acceptance. Do not continue target-host setup or live Guard/n8n/TOTP validation until explicitly resumed. Keep using the local development runtime.


## Settings discovery and connected UI

The one global settings entry is visibly labeled App settings in the footer, with a gear icon and global-preferences subtitle; do not reintroduce a duplicate sidebar navigation entry. Agent gears open separate owner settings. Keep the connected UI consistent with the established prototype styling and show unavailable credential enrollment honestly. Appearance saves use versioned backend settings. The mobile drawer is inert while hidden and contains keyboard focus when open; view changes reset content scroll. Deployment remains deferred.


## One current workspace

`/` is the canonical current UI; `/prototype.html` is a compatibility alias loading the same application. Do not reintroduce a legacy management shell or links that leave the workspace for Approvals/Memory. These flows and Activity watch controls now live in the current shell. Keep TOTP on its dedicated authorization transport, preserve exact review hashes/expiry/budgets/reconciliation and owner-scoped memory operations. Disconnect shows Connect; fictional samples require explicit selection. Deployment remains deferred.

## October 19 branch consolidation and development environment

User explicitly authorized merging all feature work into main and removing redundant branches. That consolidation is complete; PR #1 is merged. Earlier instructions to keep the PR draft/main unchanged describe the previous checkpoint and no longer govern branch consolidation. The Rocky Proxmox VM is the development machine. Use `docs/rocky-development.md` for its pinned tools and repeatable baseline checks. Production deployment and live integration acceptance remain deferred; consolidation does not authorize enabling external writes or changing execution security.

The current eight-phase grouping and its completion gates are in `docs/eight-phase-plan.md`. This Phase 1 groups detailed P01–P05; Phase 2 groups P06–P09. Do not confuse those numbers with the older detailed roadmap or mark real sandbox/host-policy acceptance complete based on fixture tests. `dev/phase2-acceptance.sh` requires actual rootless Podman and a reviewed local digest-pinned image; it does not install services or enable capabilities.

## October 20 local acceptance and documentation

Use `docs/README.md` as the documentation index and `docs/eight-phase-plan.md` as the current phase-numbering authority. Phases 0–4 are complete for local acceptance; Phase 5 app flows are next, followed by Phase 6 beta/maintenance and deferred Phase 7 production acceptance. Original P00–P25 gates remain separately tracked. Current phase changes are local-only until committed/pushed; never imply that local test evidence is fresh GitHub CI or live-provider acceptance.

The development machine is Rocky Linux 10.2 on Proxmox with rootless Podman and SELinux enforcing. Original LXC topology and older Debian checkpoints are historical deployment directions, not the current development OS or an approved production target. No production watches, model peer permissions, execution capabilities, writes, credential enrollment or service were activated by local completion. Preserve the security requirements above and the production deferral.

Latest validation is 2,494 full Rust test executions, tiny/standard 1,768 each, strict Rust 1.92 Clippy/formatting, debug/release builds and recovery, 14 frontend checks and 67 browser passes with one intentional skip. Eight host-test counts remain opt-in with separate real Phase 2 acceptance. See `docs/connected-operations.md` for write-ledger/contracts and `docs/source-aware-foundation.md` for exact peer grants, memory expiry/shared management and watch recovery. Documentation refreshes must preserve dated evidence, upstream attribution and runtime identity text instead of rewriting historical results as current claims.
