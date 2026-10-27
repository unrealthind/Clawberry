# Phase 3 — Connected operations

This is the local client/runtime milestone in the [eight-phase plan](eight-phase-plan.md). Live n8n/Guard acceptance and external writes remain separate release gates. Local fixtures never authorize a real email or mailbox change.

## Reviewed read workflows

Host-owned contracts name a fixed HTTPS endpoint, credential reference, allowed agents, reviewed operations, input bounds and projected response fields. Models supply only a workflow name and bounded input. Ownership, policy and Guard precede credential injection and transport. Redirects are disabled; requests time out after 20 seconds; JSON results are bounded to 1 MiB. No automatic application retry is dispatched.

The request carries `X-Request-ID`, `X-Agent-ID` and `X-Task-ID`. Optional response `X-N8n-Execution-ID` and `X-N8n-Audit-ID` headers must agree with matching body fields. Provided body request/agent/task fields must match the dispatched scope; identifiers are bounded. A contradictory `ok:true` plus non-null `error` is a failure. Missing remote IDs remain absent rather than being invented.

Credential redaction happens before returning results. Gateway audit/execution correlation is captured before body-field projection, so filtering model-visible response fields does not erase provenance from the runtime's result envelope/audit.

## Reviewed write workflows

Each write contract now requires an explicit `allowed_operations` list. Missing or unsupported lists fail closed. `config/n8n-write.example.toml` describes SMTP operations; `config/n8n-mailbox-write.example.toml` describes the separate mailbox-management gateway. Neither is installed or enabled automatically. Existing administrator contracts require review and this new field before use.

`write.prepare` validates ownership, the closed manifest schema and the workflow's operation allowlist. Authorization also verifies the stored manifest hash, owner and current workflow permissions before any gateway request. Dispatch rechecks the current operation allowlist, policy/Guard, verified grant, resource scope, expiry, revocation, per-operation budget and action identity.

Manifest hashes are SHA-256 over compact UTF-8 JSON with recursively sorted object keys (UTF-8 lexical order); array order is retained. The hash is independent of the enum's field declaration order and matches the canonical manifest transmitted in the request. Gateways must compute the same canonical bytes, not hash arbitrary JavaScript object insertion order or merely echo a supplied hash. Existing pending reviews prepared with the older bounded-session hash order must be replaced with fresh reviews; they fail closed rather than silently authorizing changed bytes.

Supported authorization modes remain:

- Single action: exact reviewed SMTP operation and payload, one execution.
- Bounded session: fixed mailbox/UID scope and operation budgets, at most five minutes.

The dedicated authenticated `write-authorizations` endpoint is the only code-entry path. Codes are not operation/chat data. The two gateway phases keep TOTP out of `request`; it appears only in `authorize`. Broker-held session tokens are encrypted, scoped and expiring. Known webhook/session-token/code echoes in authorization IDs are rejected before persistence; dispatch results are redacted against both credentials before correlation is stored. A burned review cannot be reused. Failures after claiming a review end in `failed`, including configuration failures before transport; partial authorization failures revoke any newly stored bearer reference.

## Results and uncertain outcomes

A write is confirmed only when `ok:true`, no non-null error, and the exact action ID plus valid gateway audit/n8n execution IDs are returned. The ledger stores those three IDs separately; the UI shows the audit and n8n IDs. Models receive fixed status fields and safe session references, not arbitrary gateway response fields or tokens.

Malformed, incomplete, conflicting or unavailable results remain uncertain. Equivalent unresolved writes stay blocked across tasks; no budget is refunded and no blind retry occurs. Human reconciliation requires `ok:true`, `final:true`, the exact action ID/payload hash and terminal `succeeded|not_applied`. `retry_allowed:true` alone is never evidence. Reconciliation records evidence and changes state; it does not dispatch a write.

End session first revokes local authority durably, then attempts remote closure. HTTP success alone does not confirm closure: the body must report success and any returned session ID must match. Broker references are removed even when remote/configuration closure fails. Closure confirmation is audited and survives reload/restart; repeating an already confirmed close does not call the gateway again or downgrade the recorded result. The UI distinguishes local revocation from unconfirmed remote closure, including legacy sessions with no remote acknowledgement.

## Write-ledger migration

The separate `writes.db` now has schema version 1, independently of schema-v9 `system.db`. Opening an unversioned write ledger migrates audit IDs that were previously stored under `gateway_action_id` into `gateway_audit_id`. Previously confirmed action IDs can be reconstructed from their bound local ID; uncertain legacy action IDs are not invented. Budgets, outcomes and spent authorizations remain unchanged. Reopening does not repeat the migration. Unknown newer versions are refused before schema changes.

Before upgrading an existing runtime, stop it and retain an administrator-private matching backup of the complete state tree: system/shared/private DBs, write/approval/review ledgers, credentials/key material, files and archives. Rollback requires its matching full backup and binary; do not run older code against the migrated write ledger.

## Reproduce local acceptance

```sh
sh dev/rocky-run.sh cargo test --locked --lib n8n::tests
sh dev/rocky-run.sh cargo test --locked --lib assistants::writes::tests
sh dev/rocky-run.sh cargo test --locked --lib security::write_sessions::tests
sh dev/rocky-run.sh cargo clippy --locked --all-targets -- -D warnings
```

The loopback HTTP fixture covers request/authorize/action/end, bearer separation, exact manifest review, durable outcomes and closure. Other tests cover owner/resource boundaries, single-action binding, bounded-session concurrency, replay, expiry/revocation, restart uncertainty, migration and reconciliation. Browser fixtures cover the dedicated TOTP transport, session budgets/results/expiry, audit display, uncertainty and End session on desktop/phone.

## Remaining live release gates

The actual reviewed n8n gateway must demonstrate shared transactional TOTP replay protection across workflows, atomic budgets, idempotent action IDs, durable execution/evidence correlation, restart recovery, expiry/revocation and exact manifest enforcement. Existing exports/static workflow state are not proof. Real read acceptance must verify the named contract and response projection. Live Guard/S14 diagnosis remains separate. No root-owned execution configuration, credentials, capability grants, service or external writes are enabled by this implementation.
