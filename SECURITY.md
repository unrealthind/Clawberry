# Security Policy

## Development status

This is an early-stage source release, not a production security certification.
Local fixtures and CI exercise implemented boundaries; live Guard, sandbox,
n8n/TOTP, HTTPS and always-on deployment require their separate acceptance gates.
See [deployment acceptance](docs/deployment-acceptance.md) and the
[publication review](docs/publication-review.md).

Do not expose the development UI publicly. `--local-ui` is token-free loopback
access and must not be reverse-proxied. Paired access and an administrator-managed
HTTPS deployment are required for remote use. The current default development LAN
HTTP flow does not encrypt credentials or traffic in transit.

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

**Please do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please report them responsibly:

Use [GitHub private vulnerability reporting](https://github.com/unrealthind/Clawberry/security/advisories/new) when enabled. If unavailable, ask a maintainer for a private reporting channel without publishing vulnerability details.

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Impact assessment
- Suggested fix (if any)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Assessment**: Within 1 week
- **Fix**: Within 2 weeks for critical issues

## Security architecture

The permanent-agent runtime uses deterministic policy, Guard review, scoped human
approvals, broker-held credentials and durable audit correlation. Janet/Harley
ownership and cross-agent approval boundaries apply to every capability. Root
policy denials cannot be overridden through an agent approval.

External writes require the dedicated n8n/TOTP path. Models receive safe references
and status, never raw credentials, TOTP codes or bearer session tokens. Session
scope, expiry, atomic budgets/replay protection and uncertain-write reconciliation
remain mandatory. See [connected operations](docs/connected-operations.md).

Memory classification is not a universal sensitive-data detector. Keep secrets
out of memory, conversations, screenshots and artifacts. Never use real private
records in public issues or test fixtures.

## Verification

Run the pinned Rust suite, frontend checks and secret scan described in
[CONTRIBUTING](CONTRIBUTING.md). Live/host-dependent acceptance is separate from
mock transports and browser fixtures. Container and release packaging must be
validated on their targets before distributing binaries; inherited container
examples do not establish production readiness.

## RaaviClaw workspace UI boundary

`/` and `/prototype.html` serve the same current workspace; `?mode=sample` explicitly selects fictional previews. Global Claude settings now accept Anthropic tokens only through a dedicated authenticated human endpoint; encrypted storage is separate from chat, memory, and ordinary settings. Saved credentials are never returned to the browser or model context. The current LAN development HTTP URL does not encrypt network transport; production HTTPS acceptance remains open. Explicit sample mode does not enroll credentials. Live settings use versioned human-only backend APIs with closed nonsecret schemas; explicit sample drafts remain browser-local. Identity/memory/skill text cannot expand runtime privileges. The normal connected workspace uses the same-origin gateway. Explicit `--local-ui` development mode permits token-free human API access only on loopback with no configured tunnel or trusted proxy headers; Host, Origin and browser fetch metadata are checked and forwarded requests rejected. Do not expose this mode through a reverse proxy. Normal mode remains paired, with tokens in memory only and cleared on disconnect/reload. Local UI access does not bypass TOTP, policy, Guard, root-of-trust or agent boundaries. Already dispatched server operations may still finish after disconnection. External write approvals remain in the dedicated authenticated n8n/TOTP path. See [current UI scope](docs/ui-workspace.md) and the deployment/security acceptance ledger before enabling production capabilities.


October 19: live settings are now server-persisted through dedicated human-only endpoints, not model operations. Closed schemas reject credential fields; common provider-secret markers are rejected in agent notes/skill content. Stored content is owner-scoped and not included in operation/audit payloads; audits retain scope/revision. `.env.local` and `.runtime/` are ignored by Git. The development launcher reads environment credentials without printing them or passing them in argv. Production web credential enrollment remains disabled pending its secure implementation. No settings field grants tool/secret/root-policy authority.
