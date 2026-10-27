# ARCHITECTURE.md

Current delivery status — October 20, 2026: [eight-phase plan](docs/eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

## Purpose

This document describes the technical architecture for the TinyClaw/Muse hybrid fork.

The system is a self-hosted, multi-agent assistant platform with:

- two permanent agents: Janet and Harley
- temporary task-scoped subagents
- proactive heartbeat/watch loops
- separate memory domains
- tool and shell execution
- sandboxed package installation
- n8n as the primary operational workflow layer
- TOTP-gated external writes
- Llama Guard pre-execution blocking
- Claude Opus escalation for blocked/sensitive actions
- a hard root-of-trust deny layer
- a desktop-first, chat-first app with Focus Threads and occasional rich cards

The architecture must preserve User's control at all times.

---

# 1. High-Level System

```text
                           User
                             │
                     Desktop / Mobile UI
                             │
                     HTTPS + WebSocket
                             │
                ┌────────────┴────────────┐
                │                         │
             JANET                     HARLEY
             WORK                     PERSONAL
                │                         │
         Heartbeat Loop            Heartbeat Loop
                │                         │
           Task Manager              Task Manager
                │                         │
          Temporary Workers        Temporary Workers
                │                         │
                └──────────┬──────────────┘
                           │
                    Agent Runtime
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
    Model Router       Tool Broker        Message Bus
        │                  │                  │
        │          ┌───────┴────────┐         │
        │          │                │         │
        │      Policy Engine   Secret Broker  │
        │          │                │         │
        │      Llama Guard          │         │
        │          │                │         │
        │       Opus Review         │         │
        │          │                │         │
        │       Approval Flow       │         │
        │          │                │         │
        │       Sandboxes           │         │
        │          │                │         │
        │         n8n ──────────────┘         │
        │          │                          │
        └──────── External Systems ───────────┘
```

---

# 2. Deployment Topology

## v1 deployment

Use one primary Linux runtime host with logically isolated agents. Current development runs on Rocky Linux 10.2 in a Proxmox VM; production host/service acceptance is deferred. The original single-LXC topology remains an optional deployment shape, not a requirement for this development VM.

```text
TinyClaw runtime host
├── API / Gateway
├── Agent Supervisor
├── Janet runtime
├── Harley runtime
├── Message Bus
├── Task Service
├── Memory Service
├── Model Router
├── Policy Engine
├── Llama Guard client
├── Approval Service
├── Secret Broker
├── n8n Bridge
├── Audit Service
├── Heartbeat Scheduler
└── Sandbox Manager
```

Janet and Harley are logically isolated and do not need separate VMs or LXCs. Rootless Podman provides task execution isolation; it does not run local reasoning-model weights. Hugging Face hosts the Guard API, not the primary assistant reasoning model. Provider activation remains separately gated.

The architecture must allow selected workers to move into:

- disposable containers
- dedicated sandboxes
- separate LXCs
- remote worker hosts

without changing agent identity.

---

# 3. Permanent Agent Model

Permanent agents are durable identities.

Each permanent agent owns:

- identity
- system prompt
- configuration
- memory
- workspace
- task queue
- heartbeat policy
- tool permissions
- secret permissions
- conversation history
- Focus Threads
- model-routing preferences

Initial permanent agents:

```text
janet  = work/business
harley = personal
```

They are peers.

Neither permanently controls the other.

---

# 4. Cross-Boundary Isolation

Janet and Harley must not directly access each other's:

- private memory DB
- private workspace
- private Focus Threads
- raw task archives
- secrets
- files
- active context

Cross-boundary information exchange must use the message/approval layer.

```text
Janet
  │
  │ request access
  ▼
Approval Service
  │
  │ User approves
  ▼
Scoped Message Exchange
  │
  ▼
Harley
```

No DB-level shortcut is allowed.

---

# 5. Agent Registry

The runtime should maintain a durable agent registry.

Suggested schema:

```text
agents
------
id
name
type
owner
status
parent_agent_id
model_profile
workspace_path
memory_db
heartbeat_enabled
created_at
destroyed_at
```

Types:

```text
permanent
ephemeral
system
```

---

# 6. Subagent Architecture

Subagents are temporary by default.

They are created for one task or one narrowly scoped objective.

Examples:

- research
- coding
- email analysis
- finance analysis
- video analysis
- document review
- travel research

## Spawn contract

A parent agent supplies:

- task
- objective
- selected context
- allowed tools
- allowed memory references
- allowed secrets
- network policy
- filesystem scope
- package-install policy
- model profile
- cost budget
- execution timeout
- completion format

Example:

```yaml
subagent:
  parent: harley
  role: video_analyst
  task: review_video_441
  model_profile: normal
  tools:
    - shell
    - file.read
    - file.write
    - package.install
  secrets: []
  filesystem:
    workspace_only: true
  network:
    allowed: true
  budget:
    usd: 2.00
```

---

# 7. Subagent Completion

Subagents return a structured completion object.

Suggested fields:

```text
status
summary
findings
confidence
sources
artifacts
recommended_memories
recommended_skills
tool_usage
cost
errors
```

After completion:

1. parent receives result
2. archive is written
3. memory candidates are evaluated
4. useful artifacts are retained
5. active worker is destroyed

---

# 8. Task System

Tasks are first-class.

Suggested core tables:

```text
tasks
task_dependencies
task_events
task_artifacts
task_sources
task_recommendations
```

Suggested task fields:

```text
id
owner_agent
created_by
title
description
priority
status
due_at
focus_thread_id
source
requires_manni
created_at
updated_at
completed_at
```

Statuses:

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

Tasks may be created by:

- User
- Janet
- Harley
- heartbeat loops
- n8n events
- subagent findings
- system recovery routines

---

# 9. Focus Threads

Focus Threads are durable, dedicated workspaces attached to a permanent agent.

Each contains:

- chat
- related tasks
- files
- artifacts
- approvals
- evidence
- subagent runs
- decisions
- timeline

They are not auto-archived.

Janet/Harley may recommend archival or cleanup during maintenance sessions, but User approves lifecycle changes.

---

# 10. Main Chat Model

Each permanent agent has one main chat.

The main chat is the primary interface.

Context should be compact and assembled dynamically from:

- recent conversation
- active priorities
- current commitments
- relevant memory
- relevant Focus Thread summaries
- unresolved approvals
- important heartbeat findings

Do not replay full historical chat context unless needed.

---

# 11. Memory Architecture

Initial storage:

```text
data/
├── system.db
├── janet.db
├── harley.db
└── shared.db
```

SQLite is preferred for v1 simplicity.

The API should abstract memory storage so migration to PostgreSQL remains possible.

---

# 12. Memory Record

Suggested semantic memory structure:

```text
id
agent_owner
content
category
confidence
source_type
source_ref
sensitivity
visibility
created_at
updated_at
last_verified_at
expires_at
```

Visibility:

```text
private
shared
task_scoped
```

Sensitive memory categories are excluded from automatic promotion by default.

---

# 13. Task Logs vs Memory

Task logs are factual execution history.

Memory is distilled knowledge.

Never treat raw logs as long-term semantic memory.

```text
task log:
"ffmpeg extracted 491 frames"

memory:
"For this type of review, one frame every 5 seconds was sufficient."
```

---

# 14. Provenance

Every important memory/fact should retain provenance.

Supported source types may include:

```text
user
file
email
api
n8n
web
subagent
heartbeat
system
manual
```

Agents should be able to answer:

- where did this come from?
- when was it last checked?
- how confident are we?
- is it inference or fact?

---

# 15. Heartbeat Architecture

Janet and Harley each have an independent scheduler.

Heartbeat configuration should include:

```text
enabled
interval
quiet_hours
domains
max_investigation_budget
notification_threshold
```

Example flow:

```text
heartbeat fires
   ↓
inspect configured sources
   ↓
detect concern
   ↓
create investigation task
   ↓
spawn workers if useful
   ↓
collect evidence
   ↓
prepare recommendation
   ↓
notify User
```

No external write is performed by heartbeat alone.

---

# 16. Model Router

Agents request reasoning profiles.

They do not bind themselves directly to provider model names.

Profiles:

```text
fast
normal
deep
```

Current mapping:

```text
fast   → Claude Haiku tier
normal → Claude Sonnet 5.5
deep   → Claude Opus 5.5
```

Normal/Sonnet should handle roughly 70% of ordinary work.

The router should support:

- task type
- difficulty
- budget
- previous failures
- safety sensitivity
- context size
- latency preference

---

# 17. Model Escalation

Suggested escalation rules:

```text
normal task
→ Sonnet

simple extraction/classification
→ Haiku

repeated failures
→ Opus

complex architecture/planning
→ Opus

Guard block
→ Opus review

conflicting evidence
→ Opus

high-impact recommendation
→ optionally Opus
```

Escalation must be logged.

---

# 18. Llama Guard

Use:

```text
meta-llama/Llama-Guard-4-12B
```

through Hugging Face Inference Providers.

Llama Guard is an execution gate.

It may hard-block tool/shell actions.

It does not make final business decisions.

---

# 19. Execution Security Pipeline

```text
Agent Tool/Shell Request
        ↓
Root-of-Trust Check
        ↓
Deterministic Policy Engine
        ↓
Llama Guard
        ↓
    SAFE / BLOCK
```

If safe:

```text
execute
```

If blocked:

```text
BLOCK
  ↓
Opus Review
  ↓
├── unsafe → deny
├── safer method → reformulate → Guard again
└── justified exception → ask User
```

Opus cannot silently override Guard.

---

# 20. Root of Trust

A small absolute deny layer exists above all model decisions.

Examples:

- disable Guard
- disable policy engine
- reveal raw secret values
- export the secret vault
- delete/tamper with audit history
- delete/tamper with security DB
- self-authorize approvals
- grant permanent unrestricted host root
- alter immutable deny rules
- directly bypass Janet/Harley isolation

These are never approvable through the app.

Manual host administration is required.

Suggested config:

```text
/etc/tinyclaw/security/root-policy.toml
```

Owned by root and non-writable by the TinyClaw service.

---

# 21. Secret Broker

The agent references a capability, not a secret value.

Example:

```yaml
credential: pricing_api
```

Execution:

```text
Agent
  ↓
Tool Broker
  ↓
Permission Check
  ↓
Secret Broker
  ↓
Inject Header/Auth
  ↓
Request
  ↓
Redaction
  ↓
Safe Response
```

Secrets must never be deliberately inserted into model context.

---

# 22. Secret Storage

Secrets should be encrypted at rest.

The runtime should store:

- secret identifier
- provider/type
- scope
- allowed agents
- allowed tools
- metadata
- encrypted value

The model may see metadata such as:

```text
credential exists
credential usable by Janet
credential supports pricing.read
```

but not the raw value.

---

# 23. Tool Broker

The Tool Broker is the only normal path from agents to external execution.

Responsibilities:

- validate tool schema
- verify agent permission
- run root-policy check
- run deterministic policy
- call Guard where required
- resolve secrets
- execute tool
- redact sensitive output
- record audit event
- return normalized result

---

# 24. Shell Broker

Shell must be mediated.

Do not give models a direct unmanaged root shell on the LXC.

The Shell Broker should support:

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

All commands are audited.

---

# 25. Agent Sandboxes

Permanent agents should have isolated workspaces.

Example:

```text
/var/lib/tinyclaw/agents/janet/
/var/lib/tinyclaw/agents/harley/
```

Workers should receive disposable task workspaces.

Examples:

```text
/var/lib/tinyclaw/tasks/<task-id>/
```

Sandbox policy controls:

- filesystem roots
- network access
- environment variables
- package installation
- process limits
- disk quotas
- CPU/memory limits
- duration

---

# 26. Package Installation

Routine packages may be installed automatically inside sandboxes.

Examples:

- ffmpeg
- yt-dlp
- jq
- ripgrep
- imagemagick
- pandoc
- python packages
- node packages

Higher-risk system changes are routed through Guard/Opus/approval.

Package installation history is part of the audit trail.

---

# 27. Skills

Skills are reusable procedures, not magical privileges.

A skill can include:

- instructions
- scripts
- schemas
- tool dependencies
- expected inputs
- expected outputs
- safety constraints

Example:

```text
skills/video-review/
├── SKILL.md
├── extract_frames.sh
├── contact_sheet.py
└── config.yaml
```

A skill may request capabilities, but may not grant them.

---

# 28. n8n Bridge

n8n is the primary execution layer for business/personal integrations.

TinyClaw should interact with n8n through a defined bridge contract.

Suggested message:

```json
{
  "request_id": "req_123",
  "agent": "janet",
  "task_id": "task_456",
  "operation": "email.reply",
  "mode": "write",
  "payload": {}
}
```

n8n returns:

```json
{
  "ok": true,
  "status": 200,
  "audit_id": "...",
  "requires_totp": true
}
```

All calls should correlate TinyClaw task IDs with n8n audit IDs.

---

# 29. TOTP Write Boundary

External writes must use the existing TOTP-gated workflow.

TinyClaw should not:

- store reusable write-session bypass tokens unless explicitly designed for it
- invent a direct-write API path
- silently reuse approvals outside scope
- execute writes if n8n says authorization is incomplete

---

# 30. Approval Service

Approval types:

```text
cross_agent
guard_exception
write_action
package_security
service_control
sensitive_tool
maintenance
```

Approval record:

```text
id
requested_by
task_id
type
reason
scope
expires_at
single_use
status
approved_by
approved_at
```

---

# 31. Audit Service

Audit records should be append-oriented.

Log:

- agent decisions
- task creation
- subagent lifecycle
- tool calls
- shell calls
- package installs
- policy decisions
- Guard decisions
- Opus reviews
- approval requests
- User decisions
- secret references
- n8n executions
- TOTP events
- memory changes
- maintenance changes

Root-of-trust rules should prohibit agent-driven audit erasure.

---

# 32. Messaging Bus

Internal messages should have structured envelopes.

Example:

```json
{
  "id": "msg_123",
  "from": "janet",
  "to": "research_184",
  "type": "delegate",
  "task_id": "task_456",
  "body": "...",
  "created_at": "..."
}
```

Suggested message types:

```text
request
response
delegate
result
status
event
approval_request
approval_result
cancel
```

---

# 33. Inter-Agent Primitives

Expose concepts such as:

```text
agent.send
agent.ask
agent.delegate
agent.spawn
agent.wait
agent.status
```

`agent.broadcast` should be restricted and not used casually.

Avoid uncontrolled agent group conversations.

---

# 34. Notifications

The runtime should distinguish:

```text
informational
recommendation
warning
approval_required
urgent
```

Janet/Harley should avoid unnecessary interruptions.

Heartbeat investigations should usually produce a concise recommendation rather than a raw event dump.

---

# 35. UI Architecture

Primary UI:

- desktop-first
- responsive
- mobile-friendly
- chat-first
- one main chat per permanent agent
- Focus Threads for dedicated work
- occasional rich cards

Suggested stack:

```text
Next.js / React
shadcn/ui
assistant-ui
assistant-ui/tool-ui
Tauri later
```

---

# 36. UI Navigation

Keep primary navigation compact.

Suggested:

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

Secondary screens:

- Memory
- Security
- n8n
- Tools
- Audit
- Costs

Global app settings are accessed once from the User footer.

---

## App and agent settings (confirmed 2026-10-18)

Global app settings and agent setup are separate scopes. Open global settings from the User profile footer or Command/Control + comma; do not duplicate this entry in workspace navigation. Global settings contain appearance, notifications, calendar preferences, gateway connection, shared Claude API configuration and shared Fast/Normal/Deep profile definitions.

Open agent setup from the control beside Janet or Harley. One page per agent consolidates identity/personality and selected memory source files, skills, credential references and access boundaries. Do not scatter these across global settings, a second personality form, or standalone workspace Skills/Vault pages. Project/conversation Files stays separate from agent identity configuration. Shared API/profile configuration does not grant agent capabilities or cross-agent access.

The normal workspace now uses live gateway data and schema-v8 backend settings. Global profile/budget and owner identity/memory/skill text saves apply to subsequent runtime requests. Explicit sample mode retains browser-only drafts. Development uses environment credentials; production web credential enrollment remains planned. Settings do not grant execution permissions or change policy. See [the implementation scope](docs/ui-workspace.md).

# 37. Rich Card Architecture

Agents return structured card payloads.

The frontend owns rendering.

Example:

```json
{
  "type": "project_update",
  "title": "Example Project Permit",
  "status": "needs_review",
  "summary": "Two city comments need a decision.",
  "actions": [
    {
      "type": "open_thread",
      "thread_id": "thread_123"
    }
  ]
}
```

No dynamic React generation for normal operation.

---

# 38. Files and Images

v1 supports:

- file upload
- image upload
- generated artifacts
- attachment association with tasks/threads
- visual analysis through agent tooling

All files should carry:

- owner
- source
- task/thread association
- checksum
- timestamps
- access scope

---

# 39. Voice Readiness

v1 is text/files/images only.

The event model should already support a future modality field:

```text
text
file
image
audio
```

Later voice flow:

```text
microphone
→ STT
→ normal conversation event
→ agent runtime
→ text response
→ TTS
```

Do not build a separate "voice agent" architecture.

---

# 40. Suggested API Surface

Possible REST/WebSocket surface:

```text
POST /api/chat/:agent
GET  /api/agents
GET  /api/tasks
POST /api/tasks
GET  /api/threads
POST /api/threads
GET  /api/activity
GET  /api/approvals
POST /api/approvals/:id/respond
GET  /api/memory/:agent
PATCH /api/memory/:id
DELETE /api/memory/:id
GET  /api/audit
GET  /api/models/usage
WS   /api/events
```

This is directional and may adapt to TinyClaw's existing gateway.

---

# 41. Suggested Internal Modules

```text
crates/
├── agent-runtime
├── agent-registry
├── message-bus
├── task-engine
├── heartbeat
├── memory
├── model-router
├── policy
├── guard
├── approvals
├── secrets
├── tool-broker
├── shell-broker
├── sandbox
├── n8n-bridge
├── audit
└── gateway
```

Prefer adapting TinyClaw's existing traits before introducing duplicate abstractions.

---

# 42. Failure Recovery

Failures should be explicit.

Flow:

```text
operation fails
  ↓
record failure
  ↓
diagnose
  ↓
retry if safe
  ↓
alternate method if safe
  ↓
escalate reasoning if needed
  ↓
notify User if intervention required
```

No silent task abandonment.

---

# 43. Cost Controls

Track:

- provider
- model
- agent
- task
- tokens
- cost
- retries
- subagent cost

Support:

- per-task budgets
- per-agent daily budgets
- deep-reasoning budgets
- notification thresholds

Cost limits must not silently compromise critical safety review.

---

# 44. Maintenance Architecture

Maintenance is a recommendation workflow.

Agent identifies:

- stale threads
- completed projects
- duplicate threads
- low-confidence memories
- old artifacts
- obsolete tasks

Agent proposes changes.

User approves.

No silent cleanup of durable user-facing workspaces.

---

# 45. North-Star Interaction

The system should behave like:

```text
User:
"What's going on with the permit?"

Janet:
- checks memory
- checks current task state
- searches relevant read-only systems
- spawns worker if useful
- gathers evidence
- provides concise recommendation
- links Focus Thread if deeper detail is needed
```

And:

```text
User:
"Review this video."

Harley:
- receives file/link
- creates task
- spawns video worker
- worker installs ffmpeg/yt-dlp if needed
- produces filmstrip/artifacts
- Harley summarizes findings
```

---

# 46. Non-Goals for v1

Do not prioritize:

- full voice assistant
- wake word
- smart-home replacement
- separate native iOS app
- separate native Android app
- complex multi-agent free-for-all rooms
- permanent specialist bot zoo
- Kubernetes
- distributed microservices
- automatic workspace deletion
- autonomous external writes

Keep v1 compact and operable.

---

# 47. Architecture Principle

When deciding between a more autonomous design and a more controllable design, prefer:

```text
proactive investigation
+
explicit authority boundaries
+
auditable execution
+
scoped approval
```

The product should be powerful because it can prepare, investigate, and execute safely—not because it ignores user control.
