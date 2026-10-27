# Phase 0 — Core Baseline

Current development environment: Rocky Linux 10.2 on User's Proxmox development VM. See [repeatable VM setup and checks](rocky-development.md). The original October 5 baseline evidence below is retained; VM revalidation is recorded separately rather than replacing upstream provenance. Production deployment acceptance remains deferred.

Latest October 20 regression evidence is 2,494 full and 1,768 each tiny/standard test executions, strict Rust 1.92 Clippy/formatting, debug/release builds and recovery, 14 frontend checks and 67 browser passes with one intentional skip. Eight host counts remain opt-in with separate actual Phase 2 acceptance. See [the eight-phase plan](eight-phase-plan.md) and `.runtime/phase4-evidence/`; earlier counts below describe their own revisions, not a regression in the latest result.

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

Reviewed on 2026-10-05. User has explicitly deferred LXC creation and deployment; current work covers the core program on the local machine.

## Source provenance

- Supplied fork: <https://github.com/unrealthind/Clawberry.git>
- Unchanged baseline revision: `61043a01a203a05d85b4e550907190005ef77a84`.
- Package: `tinyclaw` 0.1.0, Rust 2021, MIT license.
- Toolchain: Rust 1.92, pinned by `rust-toolchain.toml`.
- Feature tiers: tiny, standard, full; full is the default.
- README identifies TinyClaw as a ZeroClaw fork and links `suislanchez/tinyclaw`. Full ancestry was not investigated; the supplied fork is the inspected baseline.
- Local product instructions replaced the inherited guide; that guide is retained in `UPSTREAM_AGENTS.md` for provenance.

## Existing architecture inventory

| Area | Existing implementation | Extension strategy |
| --- | --- | --- |
| CLI | `src/main.rs`, clap subcommands | Preserve existing commands; add explicit permanent-agent commands |
| Agent loop | `src/agent/loop_.rs`, text tool-call parsing, parallel dispatch | Keep the legacy loop separate from the new conversation-only host until broker integration |
| Configuration | `src/config/schema.rs`, TOML, computed workspace paths | Retain schema; support administrator-selected config directories |
| Providers | `src/providers/traits.rs`, factories, resilient wrapper, hint router | Reuse provider abstraction; P05 now adds bounded fast/normal/deep inference |
| Channels | `src/channels/traits.rs`, CLI and external channel implementations | Later route each channel event through an explicit agent identity |
| Tools | `src/tools/traits.rs`, tool factories | Inventory every entry point before enabling tools for permanent agents |
| Memory | `src/memory/traits.rs`, SQLite/Markdown backends | Reuse backend within agent-owned namespaces; extend metadata in P14 |
| Sessions | `src/session.rs`, JSON sessions used by TUI | Keep legacy history intact; new permanent main chats use owned sessions in system storage |
| Runtime | `src/runtime/traits.rs`, native and Docker adapters | Select and verify sandbox enforcement in P09 rather than duplicating traits now |
| Security | command/path policy, encrypted secret store, gateway pairing | Foundations to extend; these do not implement the complete planned security chain |
| Gateway | Axum, body/time limits, pairing/bearer auth, webhook and WhatsApp handlers | Add authenticated owner endpoints for the new host; preserve legacy contracts |
| Daemon | supervised gateway/channels/heartbeat/scheduler components | One gateway can route both permanent identities; per-agent proactive loops remain P13 |
| Observability | `Observer`, logging/noop/OTel, health state | Add durable correlated audit events; provider/session cost estimates are not the final cost ledger |
| Deployment | Docker files, dev sandbox, user-service install helpers | Revisit after User creates the LXC; no services installed during this work |

## Existing storage inventory

- Runtime configuration normally lives under `~/.tinyclaw/config.toml`; legacy workspace under `~/.tinyclaw/workspace`.
- SQLite memory is `<workspace>/memory/brain.db`.
- `memories` contains unique ID/key, content, category, embedding, and creation/update timestamps.
- FTS5 `memories_fts` and insert/update/delete triggers maintain keyword search.
- `embedding_cache` stores cached vectors and access timestamps.
- Legacy TUI sessions are `<workspace>/sessions/<id>.json` and contain model and conversation messages.
- Cron state is `<workspace>/cron/jobs.db`; `cron_jobs` stores ID, expression, command, created/next/last run, last status, and last output. The legacy scheduler applies its shell policy and persists results; it is not a permanent-agent task scheduler.
- Heartbeat input is `<workspace>/HEARTBEAT.md`. The daemon's legacy heartbeat reads its task list on a configurable interval (minimum five minutes); these are not Janet/Harley watch loops.
- Supervised daemon component health/restart snapshots are written beside configuration as `daemon_state.json`; the runtime's health snapshot is process-owned.
- User-service helpers emit platform service files and reference the existing configuration. No service installation was performed. Skill loading in the legacy TUI/channels can clone/update `~/open-skills` automatically; the PTY fixture disables it with `ZEROCLAW_OPEN_SKILLS_ENABLED=false`. The new host never loads these skills.
- Operational stores remain legacy-owned; they are not migrated or exposed to the new permanent-agent task/model interfaces.
- Existing memory has no permanent-agent owner, factual confidence, provenance, sensitivity, or verification fields. These must be added before semantic-memory promotion.

## Baseline validation

Commands below are reproducible with the pinned toolchain and lockfile. The review used a temporary Rust installation and temporary Cargo target directories; the global shell setup was not changed.

```sh
cargo test --locked --all-targets
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
```

| Check on unchanged revision | Result |
| --- | --- |
| Default/full build through test compilation | Passed |
| Library unit tests | 1,119 passed |
| Binary unit tests | 1,120 passed; many repeat library tests because modules are compiled into both targets |
| Integration suites | 12 Docker-ignore, 7 memory comparison, 8 WhatsApp security tests passed |
| Total test executions | 2,266 passed; this is not a count of unique tests |
| Strict formatting | Failed with pre-existing diffs across multiple modules |
| Strict Clippy | Failed on pre-existing warnings; 123 library and 137 library-test diagnostics were reported |
| Tiny tier, all-targets test command | Failed because `tests/whatsapp_webhook_security.rs` imports the feature-gated gateway without gating the test |
| Live providers and target LXC | Not tested; no real credentials used, deployment explicitly deferred |

Use separate target directories for independent source checkouts. Sharing a Cargo target directory between same-name/version checkouts caused stale package artifacts during review; generated package artifacts were cleaned before final implementation validation.

## Historical constraints at the unchanged baseline

1. The legacy shell tool accepts model-supplied `approved: true`. Replace that with trusted approval state before integrating it into permanent agents.
2. Optional Composio actions execute directly. They must not become an alternate external-write path for the new platform.
3. Native execution and private database filenames are insufficient OS containment for agents with shell access.
4. No inspected implementation provides the complete root policy → deterministic policy → Guard → deep review → scoped approval chain or n8n/TOTP writes.
5. Authentication exists in the gateway; permanent-agent ownership and secret/tool capability enforcement still need their planned broker interfaces.

## P00 local-core completion

Inherited formatting is normalized and automatic Clippy fixes are applied. Remaining compatibility/style exceptions are narrowly scoped and commented: Serde signatures/default return types, retained public/instance APIs, contiguous orchestration handlers, approximate legacy telemetry/scoring, and exact fixture parsing. Overflow-prone telemetry/TUI casts now saturate safely. P05 budgets use checked integer arithmetic. The public provider reexports remain compatible.

Strict formatting and all-targets Clippy pass. Full, tiny, and standard all-targets tests pass; WhatsApp tests are correctly feature-gated. The local daemon/model fixture verifies authentication, routing, state persistence, and restart; a separate PTY fixture verifies legacy TUI rendering/input editing/clean exit without model calls. During the first PTY run, upstream initialized its community-skills checkout; that test-created checkout was verified and removed, and subsequent tests disable download/sync.

P00 is complete for the authorized local-core scope. LXC OS/resource/service/startup acceptance remains explicitly deferred until User creates the environment. This is not a deployment certification.

## October 19 — Rocky development VM revalidation

Source under test: consolidated main at `c62e27b` (implementation tip `1f97292`); this follow-up changes development tooling and documentation only. Environment: Rocky Linux 10.2, x86-64, four visible CPUs. Rust 1.92.0 and Cargo 1.92.0 are pinned; frontend validation uses Node 22.23.2 and npm 10.9.8. Isolated tool installation and the linker wrapper are documented in `docs/rocky-development.md`; native compiler/browser dependencies were installed separately through User's terminal.

Fresh VM evidence is retained locally under `.runtime/phase0-evidence/`, outside Git. Strict Rust formatting and all-targets Clippy passed. The full Rust suite passed 2,445 test executions; five host-policy/rootless-sandbox acceptance tests remain explicitly ignored. Frontend production build, all 14 unit/API checks, Prettier and dependency audit passed (zero vulnerabilities). Desktop/phone Chromium checks passed 67 tests with one intentional skip. Vite's existing large-chunk warning is informational.

Tiny and standard feature suites each passed 1,719 test executions with the same five deployment-only ignores. The full debug build passed. Both paired and token-free direct-loopback fixture smoke tests passed, including owner isolation, durable settings/tasks/threads/files/workers, authenticated events, daemon restart and complete stopped-state backup/restore with matching hashes and SQLite integrity checks. The offline TUI rendered, accepted input edits and exited cleanly. No live provider, Guard or n8n/TOTP call was required; no service or external write was enabled.

The locked optimized release build passed (9,387,056-byte local binary). Its help/startup check and token-free fixture smoke also passed, including daemon restart and complete stopped-state recovery. Shell syntax and Git whitespace checks passed. P00 is complete for this Rocky development VM; production host/service/sandbox/live-integration acceptance remains separately deferred.
