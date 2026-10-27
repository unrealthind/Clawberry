# P06–P12 guarded execution

Rocky Phase 2 local acceptance is complete: [eight-phase plan](eight-phase-plan.md). `dev/phase2-acceptance.sh` reproduces real rootless-container and protected host-policy checks; it never substitutes host execution. [Phase 3](connected-operations.md) adds reviewed integration hardening; [Phase 4](source-aware-foundation.md) adds memory/watch/peer foundations. Production/live-provider acceptance remains separate.

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

Runtime wiring now connects owned tasks to policy, Guard/deep review, exact
human approvals, encrypted credentials, rootless Podman, media artifacts and
reviewed n8n adapters. Deployment defaults remain deny-all: no model execution
or external writes are enabled merely by building this branch.

The full configuration, operation vocabulary, gateway protocol, verification
commands and remaining gates are in [P06–P12 runtime integration](p06-p12-runtime.md).

## Admission

```text
owned agent/task → immutable root deny → root-owned exact policy → Guard
    → safe: exact human approval, or explicit named read rule
    → block: deep review → safer request, denial, or exact human exception review
    → unavailable: deny
```

The reviewer cannot grant approval. Human exceptions bind the complete reviewed
request, including UUID, are single-use and expire. Neither an exception nor
TOTP can override immutable denies, host policy or an unavailable Guard.
Writes use the separate verified n8n/TOTP ledger after policy/Guard; ordinary
shell approval cannot authorize an external write.

## VM execution

Rootless Podman is installed on the Debian 13 VM. Real acceptance tests passed
for host-policy loading, absence of host files/sockets inside containers,
network denial, immutable inputs, privilege/capability limits, bounded output,
timeout/cleanup, image previews and short-video filmstrips.

The runtime requires a locally present digest-pinned image, read-only root/input
mounts, network `none`, bounded tmpfs scratch, and CPU/memory/swap/PID limits.
It has no host-shell fallback and shields cleanup from caller cancellation.
The legacy Docker/native tools remain disconnected from permanent agents.

Media inputs must belong to the selected agent/task and its Focus Thread.
Successful previews become opaque, checksummed, source-linked attachments.
Scratch files/packages are disposable; common tools persist in the approved
image. Large inputs, persistent custom package sets, worker execution grants
and restart orphan-container reconciliation still need acceptance.

## Credentials and writes

Host-terminal credential registration hides input and stores encrypted,
agent/capability-scoped references. Integration code injects values and filters
outputs; known base64/hex echoes and sensitive nested keys are redacted.
Session bearers are encrypted, expire with authorization and are removed when
a session ends. Models never receive these values.

The dedicated authenticated TOTP endpoint is separate from chat/operation
persistence, returns `no-store`, and limits attempts independently of spoofable
forwarded headers. Write manifests, review state, scopes, budgets, action IDs,
outcomes and reconciliation evidence are durable. Ambiguous outcomes block
equivalent writes, including under a newly created task, until terminal gateway
evidence is retained. End session revokes locally before remote closure.

Live Guard/deep-provider, configured n8n read/TOTP adapters, shared gateway replay
and transactional state, browser interaction and enabled-path deployment tests
remain open. See the [acceptance ledger](p06-p20-progress.md).
