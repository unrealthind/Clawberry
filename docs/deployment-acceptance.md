# Approval and always-on acceptance

October 20 clarification: PR #1 is merged; later Phase 0–4 changes remain local-only. Acceptance is recorded in [the eight-phase plan](eight-phase-plan.md), [connected operations](connected-operations.md) and [source-aware foundations](source-aware-foundation.md). Write-ledger schema/contract upgrade requirements still apply. Local fixtures do not prove live gateway/TOTP/provider/service gates or authorize production schedules, peer model grants or writes.

Reviewed October 20, 2026. The earlier product plan's deployment step 5 is Phase 7 in the [eight-phase grouping](eight-phase-plan.md), not the current Phase 5 app work. Deployment remains deferred. Do not resume target-host setup or live integration/write tests until User explicitly authorizes them. The [readiness ledger](merge-readiness.md) retains historical merge evidence and open release gates.

## Evidence from this delivery

- Full Rust tests exercise deterministic policy, immutable denials, Guard failures, exact single-use approval consumption, expiry/revocation, replay protection and uncertain-write reconciliation. These are local fixture tests, not live provider or n8n acceptance.
- A concurrent write-session regression opens two independent SQLite connections, starts competing reservations against one remaining operation, and confirms exactly one admission, zero remaining budget and one durable reservation after reopening. No external write is dispatched.
- Normal paired and explicit token-free loopback gateway smoke tests verify SSE, owner isolation, file upload/download, idempotent decisions and archived worker results. Both rehearse complete stopped fixture-state backup/restore, compare every file hash, run SQLite integrity checks and read restored decisions, files and worker artifacts.
- The Linux unit renderer now emits real newlines. Its regression checks section boundaries and the restrictive umask. Restart policy, config-directory propagation, loopback binding and shutdown timeout remain in the generated unit. This is not evidence of a running systemd service.
- The previous head's Rust 1.92 CI failed `struct_excessive_bools` after adding the explicit local UI switch. A narrow documented allowance retains the independent serialized administrator switches; the CI lint command is unchanged. Security audit, supply-chain, frontend, browser, deployment-static/image and actionlint checks passed on that previous head. Every new head still needs its own CI results.

## Inputs needed for target acceptance

The Rocky Linux 10.2 Proxmox VM is confirmed as the development machine. Production target identity, service account and runtime paths must still be confirmed when deployment resumes. Older Debian/LXC records do not establish the current production target. No input request is pending; never guess a target from unrelated SSH entries.

Use existing host credentials and broker references. Do not paste credentials or TOTP into chat, operation requests, logs or artifacts. Production web credential enrollment remains a separate unfinished capability. Local environment-based Claude success does not establish production credential readiness.

## Target execution order

1. Identify the reviewed target, OS, service user, config/state paths, current binary and image digest. Confirm SELinux requirements on Rocky; retain the corresponding host protections on other supported targets. Run the applicable non-destructive host checks. `deploy/verify-rocky10.sh` is specifically Rocky-only.
2. Stop the service cleanly. Back up the entire schema-v9 state tree, including databases, credentials/key material, owner workspaces, uploaded files and archives, into administrator-owned private storage. Do not back up only `system.db`. Preserve the stopped pre-upgrade backup and matching old binary for rollback.
3. Rehearse restoration at the same configured paths on an isolated target, preserving ownership and permissions. Verify integrity, owner settings, files/archives and consumed/uncertain authorization state. Do not run a restored copy concurrently against live write gateways or restart watches automatically.
4. Validate the reviewed rootless sandbox image and the ignored host acceptance tests. Run the live Guard SAFE, BLOCK, timeout/malformed/unavailable and safer-alternative paths. Diagnose the recorded S14 block; do not suppress categories or bypass Guard to pass acceptance.
5. Validate the named read-only n8n contract, response projection, failure handling and execution correlation. Enable no write workflow until its exact contract and test target have been reviewed.
6. Through the dedicated authenticated approval UI, review one non-destructive test action or a fixed bounded manifest. The human enters TOTP directly into that flow. Verify exact payload binding, expiry, per-operation budgets, concurrent reservation, replay rejection, revocation and terminal reconciliation before any retry of an uncertain result. Models see safe references/status only.
7. Start the user service; verify boot/linger behavior, health, SIGTERM shutdown, crash recovery, orphan cleanup, idle/peak memory and cgroup limits on the actual target. Confirm consumed approvals remain consumed and uncertain writes are not replayed.
8. Retain redacted evidence correlated to the binary/commit, target, image digest, approval/action/session IDs and n8n execution. Update release checkboxes only from observed evidence. Require green final-head CI and User's explicit release/activation approval; feature consolidation is already complete.

No target-host commands, production enrollment, security changes or live external writes were performed in this delivery.
