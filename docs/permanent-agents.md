# Phase 1 — Permanent Agent Foundation

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

Janet and Harley have separate identities, workspaces, memory databases, histories and permission records. One trusted host routes both. P02–P05 add work operations, restricted workers, archives and routing; [connected operations](connected-operations.md) and [source-aware memory/watch/peer foundations](source-aware-foundation.md) extend that local core. The security/approval contracts are implemented and tested, not globally disabled services; production execution, writes and model peer grants remain unenabled. See [the core guide](core-foundation.md).

## Run locally

```sh
cargo build --locked
target/debug/tinyclaw agents
target/debug/tinyclaw chat janet
target/debug/tinyclaw chat harley -m "Help me plan tomorrow"
```

The provider is selected by administrator configuration and the P05 profile router. New configuration defaults to Sonnet 5.5; explicit profiles or saved default/model routes can override it. Agent prompts contain no provider model IDs.

For an isolated development configuration, use:

```sh
target/debug/tinyclaw --config-dir /tmp/raaviclaw-dev agents
target/debug/tinyclaw --config-dir /tmp/raaviclaw-dev chat janet
```

The explicit directory contains `config.toml` and `workspace/`. Configure its provider/model before chat. `--config-dir` applies to runtime commands; legacy onboarding explicitly rejects it rather than silently writing a different directory. Default onboarding continues to use `~/.tinyclaw`.

`chat --state-dir <directory>` can select an administrator-owned assistant state root independently of provider configuration. Agent names are limited to `janet` and `harley`; they never become arbitrary file paths.

## Configuration and storage

Definitions are built from `agents/janet/config.toml`, `agents/harley/config.toml`, and their `AGENT.md` files. Changing these definitions requires rebuilding this foundation. The startup registry persists the current definitions while preserving conversations.

Default assistant state is `<workspace>/assistants/`:

```text
assistants/
├── data/
│   └── system.db
└── agents/
    ├── janet/
    │   └── memory/brain.db
    └── harley/
        └── memory/brain.db
```

The existing memory backend is reused with separate paths. This is a storage namespace decision; the proposed `janet.db`/`harley.db` filenames are not necessary to establish separate databases. P04 adds selective reviewed facts to an owner-private `distilled_memories` table. General semantic-memory management and shared memory remain P14 work.

The original P01 schema contained:

- `agents`: permanent identities and their definitions/storage references.
- `sessions`: exactly one main session per owner, with a revision for concurrent-write detection.
- `messages`: persisted runtime-authored sender/recipient envelopes, linked to a session.
- `audit_events`: correlated chat start, failure, and completion events.

The host selects history by owner, checks envelope ownership, supplies at most the latest 50 chat messages, and reconstructs the selected identity prompt. It commits user/reply pairs atomically after successful provider completion. Failures and timeouts leave conversation history unchanged and create audit events. Per-agent locks serialize turns in a process; revision checks reject conflicting commits across hosts.

Schema version 5 now adds the P02–P05 tables and request/result operation logs; migration preserves version-1 conversation history. See the current core guide for the complete contract.

Directory permissions are private on Unix and runtime-owned descendant symlinks are rejected. OS-level sandbox enforcement remains P09. The trusted host process has access to both domains; neither model has filesystem or database tools.

## One daemon, two assistants

```sh
target/debug/tinyclaw daemon --host 127.0.0.1 --port 8080
```

The existing gateway now supports:

| Endpoint | Behavior |
| --- | --- |
| `GET /api/agents` | Lists permanent-agent definitions and storage metadata for authenticated User |
| `POST /api/chat/janet` | Sends `{ "message": "..." }` to Janet's main chat |
| `POST /api/chat/harley` | Sends `{ "message": "..." }` to Harley's main chat |

Use the gateway's existing pairing flow and bearer authentication. These endpoints require pairing even if legacy gateway pairing is disabled. They represent User selecting a recipient, not an agent communicating across a boundary. The request body cannot choose sender, tools, credentials, or another recipient. P02 permits an owned `thread_id`/`task_id`, and P05 permits a reasoning `profile`. The operation endpoint is documented in the core guide.

Cross-agent communication is unavailable. Tools and credential grants are empty; startup fails if the compiled definitions attempt to enable them or autonomous heartbeats. Provider authentication is infrastructure-owned and never injected into the model prompt as a tool credential.

Legacy `agent`, TUI, channels, heartbeat, and integration paths retain their existing behavior for upstream compatibility. They are not the new permanent-agent execution path and still require the later policy/broker work before use as Janet/Harley operators.

## Validation and acceptance

```sh
cargo test --locked --all-targets
python3 scripts/test_assistant_smoke.py --binary target/debug/tinyclaw
cargo test --locked --no-default-features --features tiny \
  --lib --bin tinyclaw --test assistant_isolation
```

The smoke test runs a local mock provider, temporary configuration/state, and a temporary daemon; it requires no real provider credential and terminates the daemon after the test.

Acceptance coverage includes separate prompts/histories/memory, reopen persistence, message/identity validation, failed-turn audit, schema-version refusal, symlink refusal, concurrent-turn conflict detection, tool-text nonexecution, authenticated owner APIs, and both agents routed by one daemon.

This is a local core foundation. Live provider verification, append-protected audit storage, image interpretation, execution permissions, n8n/TOTP, and deployment remain later work. P02–P05 complete tasks/threads, scoped file storage, archives/workers, selective promotion, and usage/budget tracking.

## Migration and rollback

Legacy configuration remains compatible; no existing session or memory store is migrated or deleted. First use creates dedicated assistant storage at schema version 5, with additive migration from version 1. Newer schema versions are refused rather than silently opened by this runtime.

Before upgrading, back up the stopped host's complete state tree. The P01 binary refuses schema version 5, so rollback to that binary requires restoring its matching version-1 backup. Legacy pre-assistant code ignores the separate assistant tree; preserve it when reverting. Migration and restart tests retain ownership and existing histories.


## App setup scopes

Global app settings live in the User footer and hold shared Claude API/model profiles and workspace preferences. The control beside Janet or Harley opens that agent’s consolidated identity/memory files, skills, credential previews and boundary descriptions. Browser-local source files do not update runtime agent prompts or semantic memory. Credential enrollment remains disabled until the dedicated secure backend is connected. See [UI workspace scope](ui-workspace.md).


## October 19 — local product steps 1–3

Local actual Claude Fast/Normal/Deep routing is verified, the prototype defaults to live gateway data, and schema-v8 backend settings persist global preferences/profiles/budgets and separate owner identity/memory notes/skill text. Saves use revision conflicts; subsequent runtime requests use saved guidance/routing without granting new capabilities. Development uses environment credentials; production web enrollment remains planned. Thread file/decision/result completion is next, followed by unchanged Guard/n8n/TOTP/always-on deployment acceptance. This does not mark all P06–P20 complete. See [the product plan](product-plan.md).
