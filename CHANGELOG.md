# Changelog

## Unreleased — October 20 Phase 5 app work

- Add connected Files navigation with owner-scoped thread upload/download controls.
- Add shared Claude token enrollment in global settings through a dedicated authenticated endpoint, encrypted storage, metadata-only responses/audit and immediate use by new model requests.
- Add paired development launcher support; production/live deployment acceptance remains deferred.

## Unreleased — October 20 local Phases 0–4

- Verify the consolidated baseline on the Rocky Proxmox development VM with pinned Rust 1.92 and Node 22; preserve earlier branch/upstream evidence.
- Complete local security/sandbox acceptance with real rootless Podman isolation, offline packages, media, replay protection and protected default-deny host policy.
- Harden reviewed n8n operation scopes, canonical manifest hashing, secret-safe authorization, separate audit/execution correlation, uncertain-write reconciliation and durable session-closure reporting.
- Add validated selective memory promotion, audited expiry review, exact-reviewed shared-memory management, identity/expiry-bound peer approvals and source-linked read-only watch recovery.
- Latest local validation: 2,494 full Rust test executions; tiny/standard 1,768 each; strict Clippy/formatting; debug/release builds and recovery; 14 frontend checks; 67 browser checks with one intentional skip. Eight host-test counts remain opt-in with separate Phase 2 evidence.
- Phase 5 app completion is next. These changes remain local/unreleased; no production service, live write, model peer grant or production watch activation is claimed. See [the current plan](docs/eight-phase-plan.md).

## Unreleased — October 19 local product delivery

- Polish live chat with a compact growing composer, profile/model controls, independent transcript scrolling, copy feedback, starter drafts, touch-aware keyboard behavior and interrupted-response recovery.

- Retired the duplicate management UI; `/` and `/prototype.html` now serve the same current workspace.
- Integrated exact Approvals/TOTP/session controls, private/shared Memory and Activity watch controls into that workspace.
- Disconnect returns to the connection screen; sample mode requires an explicit choice.

- Restored live workspace layout/control consistency and a visible App settings footer with separate agent gears.
- Restored appearance cards, model shortcut, keyboard hints and connection/credential status sections; credential enrollment stays unavailable.
- Added durable header appearance switching with conflict feedback, view scroll reset and accessible mobile drawer focus handling.

- Default live prototype with explicit sample mode.
- Local environment-based Claude launcher and actual profile verification.
- Schema-v8 durable global and owner settings, revision conflicts, owner-only personality/memory/skill guidance and active profile/budget saves.
- Schema-v9 human-recorded thread decisions with idempotent retries and ownership validation.
- Connected thread file uploads/downloads, structured worker results and archive downloads.
- Fixed generated systemd unit newlines; added concurrent write-budget and full stopped-state recovery acceptance.
- Production credential enrollment and deployed approvals remain pending; see [product plan](docs/product-plan.md).

RaaviClaw changes are recorded below alongside preserved upstream ZeroClaw history.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Updated locked `quinn-proto` to 0.11.18 to address the six RustSec advisories reported by the corrected audit; final-head CI verification remains required.
- **Legacy XOR cipher migration**: The `enc:` prefix (XOR cipher) is now deprecated.
  Secrets using this format will be automatically migrated to `enc2:` (ChaCha20-Poly1305 AEAD)
  when decrypted via `decrypt_and_migrate()`. A `tracing::warn!` is emitted when legacy
  values are encountered. The XOR cipher will be removed in a future release.

### Added
- Chat-first assistant-ui workspace at `/prototype.html`, separate Janet/Harley conversations, responsive navigation, light/dark appearance, scoped Focus Threads, fictional workers and draft reviews.
- Task create/edit/prioritize/complete controls, optional calendar deadlines, and thread workspaces grouping tasks, filenames, decisions and sample worker handoffs.
- Global app settings in the User footer; separate consolidated agent setup for identity/memory source files, skills, credential previews and boundaries. Shared Claude API/model profiles stay global; saved model selections are browser-local drafts.
- Opt-in same-origin paired gateway view for SSE chat, internal tasks, calendar projections and read-only activity. Credential enrollment stays disabled; approvals remain in the existing authenticated app.
- Audited, owner-validated atomic `task.edit`, with title/description/thread/priority/RFC3339 due-date validation and immutable terminal tasks.
- `SecretStore::decrypt_and_migrate()` — Decrypts secrets and returns a migrated `enc2:`
  value if the input used the legacy `enc:` format
- `SecretStore::needs_migration()` — Check if a value uses the legacy `enc:` format
- `SecretStore::is_secure_encrypted()` — Check if a value uses the secure `enc2:` format

### Deprecated
- `enc:` prefix for encrypted secrets — Use `enc2:` (ChaCha20-Poly1305) instead.
  Legacy values are still decrypted for backward compatibility but should be migrated.

## [0.1.0] - 2026-02-13

### Added
- **Core Architecture**: Trait-based pluggable system for Provider, Channel, Observer, RuntimeAdapter, Tool
- **Provider**: OpenRouter implementation (access Claude, GPT-4, Llama, Gemini via single API)
- **Channels**: CLI channel with interactive and single-message modes
- **Observability**: NoopObserver (zero overhead), LogObserver (tracing), MultiObserver (fan-out)
- **Security**: Workspace sandboxing, command allowlisting, path traversal blocking, autonomy levels (ReadOnly/Supervised/Full), rate limiting
- **Tools**: Shell (sandboxed), FileRead (path-checked), FileWrite (path-checked)
- **Memory (Brain)**: SQLite persistent backend (searchable, survives restarts), Markdown backend (plain files, human-readable)
- **Heartbeat Engine**: Periodic task execution from HEARTBEAT.md
- **Runtime**: Native adapter for Mac/Linux/Raspberry Pi
- **Config**: TOML-based configuration with sensible defaults
- **Onboarding**: Interactive CLI wizard with workspace scaffolding
- **CLI Commands**: agent, gateway, status, cron, channel, tools, onboard
- **CI/CD**: GitHub Actions with cross-platform builds (Linux, macOS Intel/ARM, Windows)
- **Tests**: 159 inline tests covering all modules and edge cases
- **Binary**: 3.1MB optimized release build (includes bundled SQLite)

### Security
- Path traversal attack prevention
- Command injection blocking
- Workspace escape prevention
- Forbidden system path protection (`/etc`, `/root`, `~/.ssh`)

[0.1.0]: https://github.com/theonlyhennygod/zeroclaw/releases/tag/v0.1.0
