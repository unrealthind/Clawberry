# Janet read-only email setup

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

Prepared endpoint: `https://n8n.example.invalid/webhook/email-readonly`.
This is the previously reviewed adapter reference, not evidence that the current Rocky runtime has enrolled credentials or enabled the workflow. Do not run enrollment/live validation while deployment acceptance is deferred. Current read-correlation/projection rules and required write-contract allowlists are documented in [connected operations](connected-operations.md).
The n8n Header Auth credential uses `Authorization: Bearer <secret>`.
Register only the secret portion through the hidden terminal prompt:

```sh
target/debug/tinyclaw credential-register email_work_read --agents janet --capabilities n8n.read
```

Do not enter the `Bearer ` prefix in that prompt; the trusted transport adds it.

## Response compatibility

Keep the n8n Response Body unchanged: `={{ $json }}`. Existing agents continue
receiving the same format. The prepared contract opts into `email_readonly_v2`;
other contracts retain their explicit `ok: true` requirement.

The runtime checks read-only identity, matching operation/mailbox/UID, expected
result shape and absence of explicit failures before projecting reviewed fields.
Search results retain `mailbox_errors`; partial coverage must not be represented
as complete. Attachment downloads remain outside this initial adapter.

## Deployment gate

`deploy/n8n-email-readonly.toml` prepares the endpoint and bounds requests to
Janet, reviewed mailbox names, at most 30 messages, and attachment metadata.
Confirm actual mailbox names before installing; INBOX, Sent and Archive are
provisional allowlisted names. No wildcard mailbox access is enabled.

Install the reviewed contract as a root-owned non-runtime-writable file, reference
it in execution configuration, and allow Janet `n8n.read` only after live Guard,
policy and authenticated workflow acceptance. Keep write contracts empty.
Verify successful reads, HTTP/body failures, incorrect credentials, scope denial,
and restart credential loading. A user-run live recent INBOX max=1 request succeeded on 2026-10-06 through
the recorded Guard/deep-review/human-exception path. Gateway correlation IDs
were absent. Autonomous SAFE-path and negative/restart acceptance remain open.

## Optional execution correlation header

Keep Response Body as `={{ $json }}`. In Respond Email Data, add an HTTP
response header `X-N8n-Execution-ID` with expression value `={{ $execution.id }}`.
The runtime accepts a bounded alphanumeric execution identifier from this header
and retains it in the correlated completion audit. Conflicting header/body IDs
are failures. Live user-run acceptance on 2026-10-06 returned matching execution ID 3356
in the runtime result, confirming the deployed header and reviewed projection. No gateway audit ID is fabricated.
