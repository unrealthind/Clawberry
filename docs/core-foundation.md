# P02–P05 Core Foundation

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

The local core supports durable work, temporary workers, inspectable archives, selective memory promotion and configurable reasoning profiles. These P02–P05 IDs follow the original detailed roadmap, not the current eight phases. Later local security/integration and source-aware memory/watch/peer acceptance is documented in [the index](README.md). Development runs on the Rocky Proxmox VM; production deployment remains deferred.

## Use the core

```sh
cargo build --locked
target/debug/tinyclaw --config-dir /tmp/raaviclaw-dev agents
target/debug/tinyclaw --config-dir /tmp/raaviclaw-dev chat janet
target/debug/tinyclaw --config-dir /tmp/raaviclaw-dev chat harley --profile fast
target/debug/tinyclaw --config-dir /tmp/raaviclaw-dev work janet \
  --request '{"operation":"thread.create","title":"Example Project Permit"}'
target/debug/tinyclaw --config-dir /tmp/raaviclaw-dev work janet \
  --request '{"operation":"task.create","title":"Review comments","description":"Investigate supplied city comments"}'
target/debug/tinyclaw --config-dir /tmp/raaviclaw-dev usage janet
```

Configure provider credentials in the administrator's `config.toml` before reasoning calls. The runtime never includes those credentials in model context. New configuration defaults to Anthropic Sonnet 5.5; existing saved defaults remain compatible.

`work` also accepts `--request-file request.json`. `worker janet --spec worker.json` runs a worker and waits for its archive. CLI chat/work wait for locally spawned workers before exiting. The daemon runs them asynchronously.

## Tasks and Focus Threads

Threads have an owner, title, compact summary, and separate conversation revision/history. Tasks have owner, description, priority, optional due date/thread, creator/source, status, dependency edges, and event history. Dependency cycles and cross-owner references are refused. The human UI can edit title, description, priority, deadline and thread through `task.edit`. Terminal tasks are immutable; retries use a fresh task when necessary. `approved` and `executing` are refused until the approval/execution services exist.

Main-chat context includes at most three relevant owned thread summaries, matched against the latest message. It does not copy thread transcripts into main chat. `chat janet --thread <id>` selects the separate thread history. No operation automatically archives maintained threads.

Attachments retain filename, SHA-256 checksum, byte count, source, and task/thread ownership. They are stored under opaque IDs in the owner's workspace. CLI and authenticated binary uploads are limited to 5 MB; other gateway operations retain a 64 KiB request-body limit. Downloads verify ownership, filesystem scope and checksum and use safe attachment headers. The app supports Focus Thread uploads/downloads. File content reaches a worker only when explicitly selected as context; image interpretation is not enabled.

## Operations

Authenticated `POST /api/agents/:agent/operations` accepts the same JSON as `work`. Existing pairing/bearer authentication is required. User selects the owner; there is no model-controlled owner field or cross-agent dispatch.

| Operation | Main fields |
| --- | --- |
| `thread.create`, `thread.list` | `title` for creation |
| `thread.summary`, `thread.read` | `thread_id`; `summary` for update |
| `task.create`, `task.list`, `task.get` | `title`, `description`, optional `thread_id`; `task_id` for inspection |
| `task.edit` | `task_id`, `title`, `description`, optional `thread_id`, `priority`, optional RFC3339 `due_at`; owned nonterminal tasks only, fields and task event committed atomically |
| `task.status`, `task.metadata`, `task.dependency` | `task_id`, plus `status`, or `priority`/`due_at`, or `depends_on` |
| `file.attach`, `file.list` | `thread_id`; upload also takes optional `task_id`, `filename`, `data_base64` |
| `agent.spawn` | `spec` object below |
| `agent.status`, `agent.wait`, `agent.cancel`, `agent.retry` | `worker_id` |
| `agent.list`, `agent.recover` | No additional fields |
| `archive.read` | `worker_id`, `filename`; response contains base64 |
| `memory.candidates`, `memory.promote`, `memory.reject` | `candidate_id` for review decisions |
| `usage.list` | No additional fields |
| `reasoning.escalate` | `profile: "deep"`, nonempty `reason` |

Chat accepts `{ "message": "...", "thread_id": "...", "task_id": "...", "profile": "normal" }`; optional fields can be omitted. A selected task/thread pair must belong together. Unknown fields are refused. Thread responses return the thread session ID.

Models can request only the compiled `internal_capabilities` of their identity, using one closed `<internal_action>{...}</internal_action>` operation per turn. The host dispatches it and returns structured data. A turn permits at most eight operations. Supported model operations cover work/threads, delegation/status/wait, candidate review, and deep escalation. Administrator-only file/archive/cancellation/dependency controls are not implicitly granted to models. Requests/results are retained in owner-scoped `operation_events`; task and worker activity also has domain-specific events.

## Temporary workers

Example `worker.json` (replace `task_id` with an owned task):

```json
{
  "task_id": "owned-task-id",
  "role": "document reviewer",
  "objective": "Distill the selected permit comments",
  "context": [{"source_ref":"city-comments","source_type":"file","content":"Selected comments here"}],
  "profile": "fast",
  "budget_microusd": 100000,
  "timeout_secs": 30,
  "allowed_tools": [],
  "allowed_credentials": [],
  "retry_deep": false
}
```

Each worker has its own ID, selected context, model profile, lifetime budget, bounded timeout, and empty ephemeral workspace. It receives no parent's chat history, memory, credentials, or execution privileges. It is a temporary async model task rather than an OS sandbox process. Shell, direct network tools, package installation, filesystem/database tools, and nonempty credential grants are refused until their later security/sandbox phases.

Completion is validated JSON: `summary`, `findings`, `confidence`, and optional `sources`, `artifacts`, `memory_candidates`, `recommended_skills`, `errors`. Sources must match supplied references. Artifact names and sizes are bounded; traversal, duplicates, fabricated source references, and invalid confidence are refused. This is useful for reviewing supplied documents/code and producing text artifacts; autonomous web/video work awaits execution tools.

Parents can inspect status, await completion, cancel, and retry with a fresh worker ID. `retry_deep: true` explicitly permits one separately metered deep retry after a model-request failure. Invalid completion JSON is archived as a failure. Failed, cancelled, and timed-out workers retain evidence. Completed active worker entries and empty ephemeral directories are removed; durable worker records remain inspectable.

Startup recovers expired leases and pending archive finalization. Unexpired workers from a previous host remain uncertain until the lease expires; they are never blindly replayed. `agent.wait` refuses a worker owned by another active host process; use status polling and recovery. Gateway requests have a 30-second timeout, so poll worker status for longer work. Cancellation can leave provider billing uncertain; the reservation stays charged.

## Archives and memory candidates

Archives live at `agents/<owner>/archive/task_<task-id>/<worker-id>/`:

```text
summary.md
transcript.jsonl
tool_calls.jsonl
shell.log
artifacts/
sources.json
costs.json
memory_candidates.json
```

Directory publication is atomic; database bookkeeping can recover after publication. Transcripts retain selected delegation input, raw model output, and structured handoff/failure. Tool-call logs are empty and shell logs explicitly record that no shell capability was granted. Costs include uncertain reservations. Text artifacts retain checksums and size metadata.

Candidates are pending, promoted, or rejected. Promotion selects a single fact and writes its owner, confidence, source reference, sensitivity, visibility, and timestamps into the owner's `distilled_memories` table. Candidate state and memory insertion commit together. Sensitive candidates and excluded sensitive categories are refused. Cross-owner review and repeated promotion are refused. No transcript is dumped into memory; there is no automatic shared-memory write. General memory search/edit/delete/automatic verification remains P14.

## Model profiles and budgets

Copy [the profile example](../config/model-profiles.example.toml) beside `config.toml` as `model-profiles.toml`. All three profiles are required, with administrator-selected provider/model, configured input/output prices, and output-token limits. Default mapping is Haiku 4.5, Sonnet 5.5, and Opus 5.5, using [Anthropic's rate card](https://platform.claude.com/docs/en/models/overview), checked 2026-10-05. Agent prompts request profiles and contain no provider model IDs. Existing default provider/model and matching `model_routes` are respected when no explicit profile file exists; configure prices explicitly when changing providers/models.

The bounded inference contract is implemented for Anthropic, OpenAI, OpenRouter, and OpenAI-compatible providers. Other inherited providers fail explicitly if they cannot enforce the output bound. Compatible-provider fallback that discards history/bounds is disabled on this path. Anthropic and OpenAI now preserve full selected history for non-streaming calls.

Usage records include owner, task, worker, profile, provider/model, input/output tokens, microUSD cost, and completion/uncertainty status. Prices are configured estimates, not provider invoice reconciliation; cached-token discounts and dynamic tariffs are not modeled. Profile-specific model-route credentials stay inside infrastructure. A default-provider credential is reused only for that same provider; changing providers requires a matching credential route or a provider that permits unauthenticated use.

Before each call, an immediate SQLite transaction reserves a conservative byte-based input bound and maximum output cost. Concurrent hosts share atomic budget enforcement. Limits apply to each agent's UTC-day spend, deep UTC-day spend, lifetime task spend, and worker grant. Explicit profile selection applies to task-scoped calls and workers. A refusal never downgrades reasoning silently. Missing provider usage, timeout, interrupted requests, and uncertain failures retain the reservation. Reported usage replaces it; a bound overrun is charged and surfaced as an error.

## State and verification

`system.db` schema version 7 includes tasks/events/dependencies, threads/messages, attachments, workers/messages, archived artifacts/candidates, model usage, operation events and human-reviewed peer exchanges. Private memory databases hold owned semantic facts with confidence/provenance/sensitivity and stale status. Migration from version 1 preserves existing main histories. Newer schema versions are refused. Back up the complete stopped host state tree before upgrades. Rolling back to an older binary requires restoring its matching schema backup.

Current private-memory CRUD and exact peer-review APIs are described in the [P06–P20 ledger](p06-p20-progress.md). Automatic shared promotion, permanent-agent execution, live TOTP integration and heartbeat scheduling are not complete.

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --no-default-features --features tiny --all-targets
cargo test --locked --no-default-features --features standard --all-targets
python3 scripts/test_assistant_smoke.py --binary target/debug/tinyclaw
python3 scripts/test_tui_smoke.py --binary target/debug/tinyclaw
```

Fixtures use temporary state and local/offline providers. No live model credentials or deployment are needed. Tests cover ownership, persistence, dependency cycles, migration, lease/archive recovery, source/artifact validation, selective memory, concurrent budgets, model routing/escalation, uncertainty, cancellation, timeout, and legacy TUI interaction. Phase acceptance evidence is in [worklogs.md](../worklogs.md).

P06 onward still supplies the root policy, Guard, secrets, sandbox, n8n/TOTP, heartbeats, and full memory/app layers. The new host has no external execution/write path. Legacy execution commands retain their inherited behavior and must not be used as a shortcut around those gates.


## Human configuration API — October 19

`GET/POST /api/settings/global` persists versioned nonsecret preferences and shared profiles/budgets. `GET/POST /api/agents/:agent/settings` persists owner-only soul, memory notes, default profile and skill text packages. POST bodies are `{revision, value}` with closed types and server validation; 409 preserves a stale draft. The dedicated routes are absent from model operation dispatch. Audit stores scope/revision, not contents. `GET /api/runtime/status` returns credential presence/source metadata, routing and current-model verification evidence, never credential values. Schema v8 adds `ui_settings`; back up complete stopped state before upgrade.
