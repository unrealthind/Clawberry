# UI ADDENDUM

Current delivery status — October 20, 2026: [eight-phase plan](docs/eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

## Purpose

This addendum defines the user-interface direction for the TinyClaw/Muse hybrid fork.

It complements:

- `AGENTS.md`
- `ARCHITECTURE.md`
- `ROADMAP.md`

The UI should not feel like an enterprise dashboard with chat embedded inside it.

The primary experience is:

> **A clean, persistent AI chat with occasional rich cards and dedicated side threads when deeper work is needed.**

Janet and Harley should feel like real assistants first and operational dashboards second.

---

# 1. Product Direction

The application is desktop-first and must also work well on a phone.

Primary permanent agents:

- **Janet** — work/business
- **Harley** — personal

Each agent has:

- one main chat
- Focus Threads
- tasks
- approvals
- activity
- memory
- files
- artifacts

The main chat is where User should spend most of his time.

---

# 2. Recommended Frontend Stack

Use:

```text
Next.js / React
+
shadcn/ui
+
assistant-ui
+
assistant-ui/tool-ui
```

Later:

```text
Tauri
```

for desktop packaging.

Optional:

```text
Refine
```

only for CRUD-heavy secondary screens if it materially reduces implementation time.

Do not make Refine the primary application shell.

---

# 3. Why This Stack

## assistant-ui

Use assistant-ui for the main conversation experience.

It should provide or help provide:

- streaming messages
- conversation threads
- attachments
- markdown
- code blocks
- tool-call rendering
- message actions
- composer/input
- custom runtime/backend integration
- human-in-the-loop interaction patterns

TinyClaw remains the backend/runtime.

assistant-ui is the presentation layer.

Do not force TinyClaw to adopt assistant-ui's backend assumptions.

Create an adapter between them.

Conceptually:

```text
assistant-ui
      ↓
TinyClaw Runtime Adapter
      ↓
TinyClaw API / WebSocket
      ↓
Janet / Harley
```

## assistant-ui/tool-ui

Use tool-ui for rich structured interactions.

Examples:

- approval cards
- TOTP cards
- project update cards
- task status
- subagent activity
- Guard/security events
- file/artifact previews
- video filmstrips
- recommendation cards
- cross-agent approval cards

Agents return structured payloads.

The UI renders them.

Agents do not write React code for normal interactions.

## shadcn/ui

Use shadcn/ui for the application shell and general components.

Examples:

- sidebar
- drawers
- command menu
- dialogs
- sheets
- tabs
- badges
- buttons
- dropdowns
- forms
- tables
- popovers
- mobile navigation
- settings panels

Prefer using copied/local components so they can be freely adapted to the product.

## Tauri

Use Tauri later to wrap the responsive web application as a desktop app.

Target first:

- macOS

Later:

- Windows
- Linux

The desktop wrapper should add:

- system tray
- native notifications
- deep links
- launch on login
- file opening/sharing integration
- secure local session handling
- desktop shortcuts

The TinyClaw runtime runs on one primary Linux host. Development currently uses the Rocky Proxmox VM; always-on production hosting remains deferred. The frontend must not depend on whether the host is a VM or LXC.

The desktop app is not the agent runtime.

---

# 4. Core UI Philosophy

The interface should feel:

- clean
- compact
- calm
- fast
- conversational
- information-dense only when necessary

Avoid:

- giant dashboard home screens
- permanent analytics panels
- model usage charts in the main view
- live-agent walls
- excessive cards
- too many sidebar sections
- constant operational noise

The main screen should usually look like a chat application.

---

# 5. Main Chat

Janet and Harley each have one persistent primary chat.

Example:

```text
JANET

Janet:
I checked the Example Project project.
The city responded this morning and there are
two comments worth your attention.

┌───────────────────────────────────┐
│ Example Project Permit                    │
│ Needs review                      │
│                                   │
│ 2 city comments require a decision│
│                                   │
│ [Review]          [Open Thread]   │
└───────────────────────────────────┘

User:
What do you recommend?

Janet:
I'd deal with the mechanical comment first...
```

Most messages should remain plain conversation.

Cards appear when structure improves usefulness.

---

# 6. Minimal Primary Navigation

Keep the main navigation small.

Recommended:

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

`More` may contain:

- Memory
- Security
- n8n
- Tools
- Audit
- Costs

Global app settings live in the User footer, not a duplicate primary-navigation item.

Do not expose every internal subsystem in the permanent sidebar.

---

## App and agent settings (confirmed 2026-10-18)

Global app settings and agent setup are separate scopes. Open global settings from the User profile footer or Command/Control + comma; do not duplicate this entry in workspace navigation. Global settings contain appearance, notifications, calendar preferences, gateway connection, shared Claude API configuration and shared Fast/Normal/Deep profile definitions.

Open agent setup from the control beside Janet or Harley. One page per agent consolidates identity/personality and selected memory source files, skills, credential references and access boundaries. Do not scatter these across global settings, a second personality form, or standalone workspace Skills/Vault pages. Project/conversation Files stays separate from agent identity configuration. Shared API/profile configuration does not grant agent capabilities or cross-agent access.

The normal workspace now uses live gateway data and schema-v8 backend settings. Global profile/budget and owner identity/memory/skill text saves apply to subsequent runtime requests. Explicit sample mode retains browser-only drafts. Development uses environment credentials; production web credential enrollment remains planned. Settings do not grant execution permissions or change policy. See [the implementation scope](docs/ui-workspace.md).

# 7. Janet and Harley Switching

Janet and Harley should feel strongly distinct while remaining in the same application.

Switching agents should be immediate.

Each should retain:

- last conversation position
- drafts
- active Focus Thread
- unread/important state
- pending approvals
- contextual navigation state

Do not merge their conversations.

Do not place both agents in the same default conversation.

---

# 8. Focus Threads

Focus Threads are dedicated side conversations/workspaces.

Use for:

- a business project
- a permit issue
- a research project
- a trip
- a home project
- a video review
- a complex investigation

Example:

```text
Janet
├── Main Chat
├── Example Project Permit
├── Jagmeet Liquor Store
├── Pricing Workflow
└── Supreme Office
```

Harley:

```text
Harley
├── Main Chat
├── Vancouver Trip
├── House Tasks
└── Video Review
```

The main chat remains the default.

---

# 9. Focus Thread Contents

A Focus Thread may contain:

- dedicated conversation
- related tasks
- files
- artifacts
- evidence
- approvals
- subagent runs
- decisions
- source links
- activity timeline

These details should not all be visible simultaneously.

Use tabs, drawers, or contextual panels when needed.

The conversation remains primary.

---

# 10. Main-Chat Summaries

Important side-thread information may be surfaced into the main chat as a compact summary.

Example:

```text
┌─────────────────────────────────────┐
│ Jagmeet Liquor Store                │
│ Waiting on client equipment info    │
│                                     │
│ Last checked: 22 min ago            │
│ Confidence: 96%                     │
│                                     │
│ [Open Thread]   [Review Evidence]   │
└─────────────────────────────────────┘
```

Do not copy the entire Focus Thread into the main chat.

Use compact summaries.

---

# 11. Rich Card System

Rich cards are predefined frontend components.

Agents emit structured card data.

Agents should never generate arbitrary React/HTML for ordinary cards.

Recommended initial types:

```text
project_update
recommendation
task
approval
totp
subagent_status
cross_agent_request
security_event
file
artifact
video_filmstrip
memory
generic_info
```

---

# 12. Card Usage Rule

Cards should be occasional.

Preferred distribution:

```text
80% normal conversation
15% predefined rich cards
5% generic/custom structured presentation
```

This is directional, not a strict metric.

If plain text is clearer, use plain text.

---

# 13. Card Payloads

Example:

```json
{
  "type": "project_update",
  "title": "Example Project Permit",
  "status": "needs_review",
  "summary": "Two city comments need your decision.",
  "confidence": 0.96,
  "actions": [
    {
      "type": "open_thread",
      "thread_id": "thread_123"
    },
    {
      "type": "review_evidence",
      "task_id": "task_456"
    }
  ]
}
```

The frontend decides how this looks.

---

# 14. Generic Information Card

A generic schema-driven card is acceptable for uncommon structured information.

Example:

```json
{
  "type": "generic_info",
  "title": "Permit Review",
  "fields": [
    {"label": "Confidence", "value": "94%"},
    {"label": "Source", "value": "City email"},
    {"label": "Recommended next step", "value": "Review mechanical comment"}
  ]
}
```

This reduces the need to build hundreds of one-off components.

---

# 15. Approval Cards

Approvals must be obvious and compact.

Example:

```text
┌─────────────────────────────────────┐
│ Approval Required                   │
│                                     │
│ Janet wants to send a client email. │
│                                     │
│ Action: email.reply                 │
│ Scope: one message                  │
│ Write path: n8n + TOTP              │
│                                     │
│ [Review]              [Continue]    │
└─────────────────────────────────────┘
```

The next step may open the TOTP flow.

---

# 16. TOTP Card

Example:

```text
┌─────────────────────────────────────┐
│ TOTP Required                       │
│                                     │
│ Send prepared reply to client       │
│                                     │
│ [ _ _ _ _ _ _ ]                    │
│                                     │
│ [Cancel]              [Authorize]   │
└─────────────────────────────────────┘
```

Do not expose secret values in the UI.

---

# 17. Guard / Opus Card

Security events should be understandable without showing excessive internal details.

Example:

```text
┌─────────────────────────────────────┐
│ Action Blocked                      │
│                                     │
│ Llama Guard blocked a shell command.│
│                                     │
│ Opus reviewed the request and found │
│ a safer alternative.                │
│                                     │
│ No approval is required.            │
│                                     │
│ [View Details]                      │
└─────────────────────────────────────┘
```

If approval is needed:

```text
[Approve Once] [Deny]
```

Root-of-trust denials should never show an approval button.

---

# 18. Cross-Agent Request Card

Because Janet ↔ Harley communication requires User approval:

```text
┌─────────────────────────────────────┐
│ Cross-Agent Request                 │
│                                     │
│ Janet wants to ask Harley:          │
│                                     │
│ "Is User free Thursday 6–9 PM?"    │
│                                     │
│ Scope: availability only            │
│                                     │
│ [Deny]                [Allow Once]  │
└─────────────────────────────────────┘
```

Do not grant broad memory access.

---

# 19. Subagent Status Card

Only show subagent status when it is relevant to the conversation.

Example:

```text
┌─────────────────────────────────────┐
│ Video Review                        │
│                                     │
│ Video Analyst                       │
│ Extracting frames...                │
│                                     │
│ ffmpeg ✓                            │
│ 284 / 550 frames                    │
│                                     │
│ [Open Activity]                     │
└─────────────────────────────────────┘
```

Do not permanently show every worker in the main chat.

---

# 20. Video Filmstrip Card

For media review:

```text
┌─────────────────────────────────────┐
│ Video Review Complete               │
│                                     │
│ [frame] [frame] [frame] [frame]     │
│ [frame] [frame] [frame] [frame]     │
│                                     │
│ 12 notable moments found            │
│                                     │
│ [View Filmstrip] [Open Thread]      │
└─────────────────────────────────────┘
```

---

# 21. Files and Images

The composer should support:

- file drag-and-drop
- image paste
- image upload
- document upload

Attachments should appear naturally in the conversation.

Files may later become associated with:

- task
- Focus Thread
- artifact
- memory source

---

# 22. Composer

Keep the message composer simple.

Recommended:

```text
[ + ]  Message Janet...                     [Send]
```

Optional compact actions may appear near it:

```text
Attach
Camera/Image
```

Avoid exposing model/provider configuration in the default chat UI.

Model routing belongs to the runtime.

Advanced controls may live in settings/developer mode.

---

# 23. Streaming Activity

During a task, show short useful state messages.

Example:

```text
Janet is checking project records...
```

then:

```text
Janet asked a research worker to inspect the city response...
```

Avoid dumping raw chain-of-thought or internal reasoning.

Good activity:

```text
Searching email
Checking project notes
Reviewing permit response
```

---

# 24. Activity Screen

Activity is a secondary screen.

It may show:

- running tasks
- temporary subagents
- completed tasks
- failed tasks
- heartbeat investigations
- recent tool use
- recent n8n executions

---

# 25. Tasks Screen

Tasks should be simple and operational.

Useful fields:

- title
- owning agent
- status
- priority
- due date
- Focus Thread
- blocker
- recommendation

Avoid turning this into a full enterprise project management suite in v1.

---

# 26. Approvals Screen

Show all pending decisions in one place.

Categories:

- external write
- TOTP
- Guard exception
- cross-agent request
- sensitive shell/tool action
- maintenance recommendation

Keep actions explicit.

---

# 27. Memory Screen

Memory should be inspectable but not part of the primary chat UI.

Support:

- search
- Janet / Harley filtering
- source
- confidence
- sensitivity
- private/shared
- verification date
- edit
- delete

Show provenance clearly.

---

# 28. Maintenance UI

Maintenance should feel like a review conversation, not a database cleanup panel.

Example:

```text
Janet:
We haven't done maintenance in 12 days.

I recommend:

1. Archive the completed Pricing API thread.
2. Keep Example Project pinned.
3. Reverify 3 low-confidence memories.
4. Remove 2 obsolete tasks.

[Start Maintenance Review]
```

User approves meaningful lifecycle changes.

---

# 29. Desktop Layout

Recommended default desktop layout:

```text
┌──────────────┬──────────────────────────────┐
│              │                              │
│ Janet        │          MAIN CHAT           │
│ Harley       │                              │
│──────────────│                              │
│ Threads      │                              │
│ Tasks        │                              │
│ Activity     │                              │
│ Approvals    │                              │
│ More         │                              │
│              │                              │
└──────────────┴──────────────────────────────┘
```

Do not show a third permanent right sidebar by default.

Contextual details may open as:

- drawer
- modal
- side panel
- Focus Thread

---

# 30. Mobile Layout

Mobile should remain chat-first.

Suggested:

```text
┌───────────────────────┐
│ Janet            •••  │
├───────────────────────┤
│                       │
│        chat           │
│                       │
│   occasional cards    │
│                       │
├───────────────────────┤
│ Message Janet...      │
├───────────────────────┤
│ Chat Threads Tasks More│
└───────────────────────┘
```

Prioritize:

- chat
- approvals
- notifications
- Focus Thread summaries
- tasks
- file/image sharing

---

# 31. Notifications

Desktop/mobile notifications should be concise.

Examples:

```text
Janet
Example Project needs your review.
2 city comments require a decision.
```

```text
Harley
I finished reviewing the video.
12 notable moments found.
```

Do not send a notification for every heartbeat or subagent event.

---

# 32. Command Palette

Useful desktop commands:

```text
⌘ K

Talk to Janet
Talk to Harley
Open Example Project
Show approvals
Show active tasks
Search memory
New Focus Thread
```

---

# 33. Search

Global search should eventually include:

- conversations
- Focus Threads
- tasks
- memories
- files
- artifacts
- archived subagent runs

Search results must respect Janet/Harley boundaries.

---

# 34. Theme

Default visual direction:

- dark mode first
- clean neutral surfaces
- restrained accents
- high text readability
- subtle agent identity cues
- minimal animation
- premium but not decorative

Janet and Harley may have slightly different accent/personality cues, but should clearly belong to the same product.

---

# 35. Accessibility

Support:

- keyboard navigation
- visible focus states
- high contrast
- scalable text
- semantic controls
- reduced motion
- screen-reader labels

Do not rely only on color for status.

---

# 36. Performance

Prioritize:

- streaming responses
- WebSocket/SSE updates
- lazy loading history
- virtualization for long threads
- thumbnail previews
- asynchronous artifact loading

Do not load years of conversation history at once.

---

# 37. Backend/UI Contract

The frontend should not contain agent reasoning rules.

Backend controls:

- agent identity
- model routing
- permissions
- memory
- task logic
- Guard
- Opus review
- approval state
- n8n
- TOTP state
- secrets

Frontend controls:

- presentation
- navigation
- interaction
- user confirmation
- rendering structured events

---

# 38. Suggested Event Types

The backend may stream normalized events such as:

```text
message.delta
message.complete
task.created
task.updated
subagent.started
subagent.completed
approval.requested
approval.resolved
guard.blocked
opus.reviewed
artifact.created
thread.updated
heartbeat.finding
notification.created
```

---

# 39. Card Versioning

Structured cards should be versioned.

Example:

```json
{
  "type": "project_update",
  "version": 1,
  "data": {}
}
```

Unknown card types should gracefully fall back to readable text or generic structured presentation.

---

# 40. UI Security

The frontend must not receive raw secret values.

The frontend may receive:

- credential ID
- credential display name
- scope
- status
- last used

but not the underlying token/key.

---

# 41. Approval Integrity

Approval UI must clearly show:

- requesting agent
- action
- target
- reason
- scope
- whether TOTP is required
- whether approval is single-use
- expiration if applicable

Prefer explicit controls such as:

```text
Approve Once
Deny
Authorize + TOTP
```

---

# 42. Voice Readiness

v1 is text/files/images only.

Do not make the v1 interface microphone-first.

However, reserve architecture for future voice.

Voice must use the same:

- conversation
- agent
- memory
- task
- approval
- Focus Thread

systems.

Voice is another interface to Janet/Harley, not another set of agents.

---

# 43. Do Not Build in v1

Avoid:

- complex dashboard home page
- custom charting system
- arbitrary agent-generated React
- native mobile rewrite
- 3D agent visualization
- giant monitoring console
- permanent model picker
- token graphs in main chat
- multi-pane IDE-style layout
- visual agent graph as default navigation
- free-form multi-agent group rooms
- overly elaborate animations

---

# 44. v1 Screen List

Required:

```text
1. Janet Main Chat
2. Harley Main Chat
3. Focus Threads
4. Focus Thread Detail
5. Tasks
6. Activity
7. Approvals
8. Global App Settings (User footer)
9. Agent Setup (beside each agent)
```

Secondary screens may arrive incrementally:

```text
Memory
Security
n8n
Audit
Tools
Costs
```

---

# 45. Recommended Build Order

## UI Phase A

- app shell
- Janet/Harley switcher
- main chat
- streaming
- composer
- file/image attachments

## UI Phase B

- Focus Threads
- thread summaries
- thread navigation

## UI Phase C

- rich-card renderer
- project update
- task
- approval
- TOTP
- file/artifact

## UI Phase D

- Tasks
- Activity
- Approvals screens

## UI Phase E

- memory/admin secondary screens
- search
- command palette

## UI Phase F

- Tauri desktop wrapper
- notifications
- tray
- deep links

---

# 46. Design Test

Before adding UI, ask:

> Does User need to see this all the time?

If no:

- put it behind a detail view
- place it in Activity
- put it in a Focus Thread
- expose it through a card only when relevant

The default screen should remain calm.

---

# 47. Final UI North Star

The application should feel like:

```text
A very capable chat assistant
that occasionally reveals
exactly the right controls,
evidence, task state,
or approval interface
when needed.
```

Not:

```text
A complex admin dashboard
that happens to contain a chatbot.
```

Janet and Harley should feel present, capable, and proactive without the interface constantly exposing the machinery underneath them.


## October 19 thread and recovery delivery

Step 4 is complete locally: authenticated thread file upload/download, schema-v9 durable human decisions with idempotent retries, and structured worker results/archive downloads. Local validation: 2,445 Rust tests and 14 frontend checks passed; 43 browser checks passed with one intentional skip; five existing host sandbox Rust tests remain ignored. Both gateway modes passed complete stopped fixture-state backup/restore. The Linux service unit newline bug is fixed, and concurrent write-budget admission is tested across independent connections.

Step 5 is deferred by User; target VM, live Guard/n8n/TOTP and always-on acceptance remain open. Production credential enrollment remains unfinished. Final-head CI is required; At that historical checkpoint PR #1 was draft and main was unchanged; the later authorized consolidation superseded that restriction. See [deployment acceptance](docs/deployment-acceptance.md).
