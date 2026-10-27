# P06–P12 runtime integration

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

Reviewed 2026-10-06, America/Edmonton. The local implementation is connected;
Live Guard BLOCK/deep-review/exact-human-exception and bounded email read
acceptance have passed; production acceptance remains open for the SAFE path,
Guard S14 diagnosis, the always-on service and deployed write contracts. No live external write has been performed.

## Core versus skills

The core owns transport, identity/task binding, permission admission, encrypted
credentials, Guard/deep review, approvals, TOTP handoff, write-session budgets,
audit, and reconciliation. Each integration has an administrator-reviewed
contract and an n8n workflow. Skills describe how to use those named operations;
they contain neither webhook credentials nor independent write transports.

## Host configuration

`deploy/setup-execution-host.sh` installs rootless Podman prerequisites and a
root-owned default-deny policy. It does not enable agent execution or writes.
The current development VM runs Rocky Linux 10.2 on Proxmox with rootless Podman,
SELinux enforcing and the installed default-deny policy. Earlier Debian 13 records
describe a historical host checkpoint, not the current development OS. Production
target sizing and service acceptance still require confirmation.

Build the administrator-reviewed image:

```sh
podman build -f deploy/Containerfile.sandbox -t localhost/raaviclaw-sandbox:acceptance .
podman image inspect localhost/raaviclaw-sandbox:acceptance --format '{{.RepoDigests}}'
```

Install a reviewed copy of `config/execution.example.toml` at
`/etc/tinyclaw/security/execution.toml`, replacing the placeholder image digest.
All ancestors/config files must be root-owned, without symlinks or group/world
write permission. The runtime must be non-root. `read_contracts` and
`write_contracts` reference similarly protected files. `agent_capabilities`
explicitly enables a closed set of broker operations per permanent agent;
default lists are empty. Human approval/reconciliation operations cannot be
added to a model's capability list. Legacy CLI/TUI model registries are empty; the model-controlled shell `approved` flag has been removed. Raw legacy adapters remain only as trusted library/fixture code.

Register credentials from a human terminal; their values are requested with
terminal echo disabled, never as command arguments:

```sh
target/debug/tinyclaw credential-register huggingface_guard --agents janet,harley --capabilities guard.assess
target/debug/tinyclaw credential-register email_work_read --agents janet --capabilities n8n.read
target/debug/tinyclaw credential-register email_write_gateway --agents janet --capabilities n8n.write
```

Use the same `--config-dir` as the daemon if using a non-default configuration.
Values live only in encrypted private storage. Registration is a host operation,
not a model/API tool. Session tokens are separately encrypted, expire with the
session, are never returned to models/UI, and are removed on End session.

## Execution and review

The authenticated `work` CLI and `/api/agents/:agent/operations` API support:

- `execution.approve`: exact request plus `reviewed_hash` of its canonical JSON
  payload; grants one execution for at most five minutes. Human only.
- `execution.revoke`: `task_id`, `approval_id`. Human only.
- `execution.run`: exact `ExecutionRequest`, with selected agent/task,
  capability/operation/resource, UUID, and payload.
- `execution.approve_exception`: exact request, `review_id`, `reviewed_hash`.
  Human only; requires a recorded deep-model exception recommendation for the
  complete request, including action identity. Does not bypass root policy.

Every operation checks owned task state. A hard-denied operation cannot reach
Guard, inference, approval consumption, or execution. Other actions require the
host allowlist and live Guard. Guard failure is a denial. Guard block calls the
configured `deep` profile and records the advisory result. A safer alternative
must retain identity/task/capability/operation/resource, use a fresh UUID, and
re-enter the normal broker. The reviewer cannot grant an exception. A separate
exact human approval is consumed atomically if Guard blocks that same reviewed
request again. Guard unavailability never accepts an exception.

Supported sandbox payloads:

- `shell` / `shell.execute`: `{ "command": "..." }`
- `package` / `package.search|install|remove|list`: `{ "package": "name" }`
  (omit package for list). Installation uses administrator-bundled wheels with
  `--no-index`; no package/repository network or host installation is available.
- `media` / `media.process`: `{ "attachment_id": "...", "kind": "image|video" }`.
  The attachment must belong to the selected task, which has an owned Focus
  Thread. The result is a checksummed JPEG attachment with source provenance.

Containers use a digest-pinned local image (`--pull=never`), network `none`, no
Linux capabilities, no new privileges, private PID/IPC namespaces, PID/CPU/memory
and swap limits, read-only image/input mounts, and a 256 MiB scratch workspace.
Only bounded, runtime-authored task staging directories are mounted. Output is
bounded to 64 KiB per stream while still drained; dispatch has a timeout, forced
container cleanup, and two concurrent process-local slots. HTTP cancellation
cannot cancel the internal cleanup task. No host-shell fallback exists.

Scratch package installs and shell files are disposable. Common utilities are
persistent in the approved image. Persistent user-installed package sets,
worker execution grants, restart orphan-container reconciliation, and large
media/n8n-attachment ingestion still need separate acceptance; this does not
claim those features are finished.

## n8n reads

`n8n.read` accepts `task_id`, a registered `workflow` name, bounded `payload`,
and optional `request_id` for a reviewed Guard exception. Arbitrary URLs,
credentials, connection parameters, unknown fields, peer access and unreviewed
operations are rejected. Root policy may grant autonomous reads only by exact
name. Responses require explicit `ok: true` by default. The administrator-only
`email_readonly_v2` contract accepts the existing email workflow format after
checking read-only identity, matching operation/mailbox/UID, result shape and
absence of explicit errors. Only reviewed `response_fields` survive projection. Known plaintext/base64/hex credential
representations and sensitive nested keys are redacted. Transport errors and
HTTP-200 body errors are audited; the adapter never silently retries.

## TOTP writes

1. `write.prepare(task_id, workflow, manifest)` returns an immutable reviewed
   manifest, hash, ID and five-minute review expiry.
2. User reviews its exact single email or bounded mailbox session in Approvals.
3. The app sends the code only to the dedicated authenticated
   `POST /api/agents/:agent/write-authorizations` endpoint with review ID/hash.
   Its body is never persisted as an operation/chat event. Responses are
   `no-store`; authorization attempts have a separate five-per-minute limit.
4. Trusted code sends request/authorize phases to the reviewed n8n gateway.
   It requires matching agent/task/broker/manifest and a bounded expiry, stores
   the returned bearer token in the Secret Broker, and records a verified grant.
   A review is atomically burned before transport; failure needs a fresh review.
5. `write.run(task_id, session_id, action, payload, request_id optional)` still
   passes host policy and Guard, then atomically reserves the exact scoped
   operation budget and unique action ID before dispatch.
6. `write.status` returns expiry, remaining budgets and action outcomes.
   `write.end` revokes locally before attempting gateway closure and removes
   the local bearer reference. Revocation cannot undo an already dispatched act.
7. Unknown/failed/uncorrelated gateway outcomes remain uncertain. Equivalent
   unresolved writes are blocked across tasks for the same owner. Human-only
   `write.reconcile` requires terminal gateway evidence binding the action ID
   and payload hash, persists a correlated evidence audit, and never dispatches
   a retry itself. A `retry_allowed` flag is not used as proof.

The single-email manifest accepts bounded explicit recipients/header/body fields;
caller-controlled connection settings, raw paths/URLs and email attachments are
not enabled. Bounded mailbox sessions enforce exact mailbox/UID scope and
per-operation budgets. Other integrations need their own reviewed adapters.

## Required deployed gateway contract

`config/n8n-write.example.toml` specifies the fixed HTTPS endpoint, named
credential, broker identity and allowed agents. It requires an adapter around
existing exports; it does not assert that an arbitrary existing webhook already
implements this protocol.

- `request`: accepts runtime request ID, agent/task/broker, manifest/hash. Returns
  `ok:true`, gateway `request_id`, and the identical `manifest_hash`.
- `authorize`: accepts gateway request ID, manifest hash and human `totp`.
  Returns `ok:true`, matching `agent`, `task_id`, `broker_identity`,
  `manifest_hash`, `session_id`, `session_token`, Unix `expires_at`, and `audit_id`.
- `action`: uses the broker-held session token as bearer authorization; accepts
  gateway session ID, runtime action ID, agent/task, operation and exact payload.
  A confirmed success returns `ok:true` and the same `action_id`, plus durable
  audit/execution IDs. Exact SMTP drafts must remain server-locked; the deployed
  adapter must translate the reviewed draft to the existing empty-body execute
  contract without accepting a replacement draft.
- `end`: revokes the named gateway session.
- `status`: uses the webhook credential and returns terminal `final:true`, exact
  action ID/payload hash, and `status:succeeded|not_applied`. Non-final/unknown
  results cannot authorize replay.

The gateway must enforce shared transactional TOTP replay protection across
workflows, scope/budget atomicity, idempotent action IDs, expiry/revocation,
restart recovery and durable outcome evidence. Workflow static data and a
rolling audit window alone do not satisfy this contract. These properties need
live deployed tests before any external write capability is enabled.

## Verification

Run the normal locked Rust suites, strict Clippy, frontend build/tests, and the
localhost-only assistant smoke test. Explicit VM checks use:

```sh
RAAVICLAW_SANDBOX_IMAGE='<reviewed RepoDigest>' cargo test --locked --test sandbox_acceptance -- --ignored --test-threads=1
```

The tests deliberately opt in to real Podman. They check root policy ownership,
network/host isolation, immutable inputs, capability/privilege boundaries,
timeout/cleanup, output limits and image/video decoding. Live Guard/deep-provider,
n8n/TOTP and browser interaction acceptance remain distinct from these tests.

## Live read-only acceptance and current deployment

Janet’s reviewed email contract is enabled on the VM. Both HF and email webhook
credentials were registered via hidden terminal input. The n8n response body
remains unchanged for existing clients; the optional X-N8n-Execution-ID response
header is captured and durably correlated. See [email setup](n8n-email-readonly-setup.md).

Guard reports BLOCK/S14 on the bounded read. Exact human exceptions allowed two
user-run reads; no category or policy override was introduced. Category-only Guard
reports are now audited. Anthropic review works with the updated sampling and
mixed-content parsers. Persistent reasoning credentials for a service still need
secure setup; the terminal-only environment variable is not a service deployment.

Live consumed-approval state and correlated runtime events were verified without
reading private memory/email content. Two temporary localhost gateway process
starts passed health/state preservation and were stopped after acceptance. No
user-level systemd unit is installed; these checks do not prove full daemon service
restart, notifications, channels or scheduler behavior. Writes remain disabled.

Host diagnostics (no email dispatch):

```sh
cargo run --locked --offline --example guard_acceptance -- '<owned task ID>'
python3 deploy/check-gateway-restart.py --request-id '<consumed request ID>'
```

These use the default runtime configuration/state; localhost gateway logs are
suppressed to prevent pairing secrets from entering artifacts.


## October 19 — local product steps 1–3

Local actual Claude Fast/Normal/Deep routing is verified, the prototype defaults to live gateway data, and schema-v8 backend settings persist global preferences/profiles/budgets and separate owner identity/memory notes/skill text. Saves use revision conflicts; subsequent runtime requests use saved guidance/routing without granting new capabilities. Development uses environment credentials; production web enrollment remains planned. Thread file/decision/result completion is next, followed by unchanged Guard/n8n/TOTP/always-on deployment acceptance. This does not mark all P06–P20 complete. See [the product plan](product-plan.md).


## October 19 thread and recovery delivery

Step 4 is complete locally: authenticated thread file upload/download, schema-v9 durable human decisions with idempotent retries, and structured worker results/archive downloads. Local validation: 2,445 Rust tests and 14 frontend checks passed; 43 browser checks passed with one intentional skip; five existing host sandbox Rust tests remain ignored. Both gateway modes passed complete stopped fixture-state backup/restore. The Linux service unit newline bug is fixed, and concurrent write-budget admission is tested across independent connections.

Step 5 is deferred by User; target VM, live Guard/n8n/TOTP and always-on acceptance remain open. Production credential enrollment remains unfinished. Final-head CI is required; At that historical checkpoint PR #1 was draft and main was unchanged; the later authorized consolidation superseded that restriction. See [deployment acceptance](deployment-acceptance.md).
