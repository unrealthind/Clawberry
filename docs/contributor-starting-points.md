# Contributor starting points

RaaviClaw is open for development. Start with a small issue you can reproduce and
verify. Please discuss architecture and security changes before implementation.

| Area | Read first | Useful contributions |
| --- | --- | --- |
| Chat and app | [App guide](../app/README.md), [UI scope](ui-workspace.md) | Accessibility, error recovery, attachment/image flows, evidence presentation |
| Runtime | [Core foundation](core-foundation.md), [permanent agents](permanent-agents.md) | Ownership regressions, task/worker lifecycle, recovery |
| Security and execution | [Execution foundation](execution-foundation.md), [SECURITY](../SECURITY.md) | Fail-closed behavior, scoped grants, rootless sandbox acceptance |
| Integrations | [Connected operations](connected-operations.md) | Reviewed read contracts, TOTP session/replay/reconciliation acceptance |
| Deployment | [Deployment acceptance](deployment-acceptance.md) | HTTPS, service lifecycle, resource measurements, backup/restore |
| Documentation and packaging | [README](../README.md), [publication review](publication-review.md) | Clean-machine setup, artifact validation, accurate feature scope |

The runtime lives primarily in `src/`; the frontend is under `app/src/`. Existing
abstractions should be extended where practical. The crate/executable retains the
`tinyclaw` name. The suggested structure in AGENTS.md is a direction, not the
current directory layout.

Use fictional fixtures. Never solve an acceptance failure by disabling Guard,
bypassing approvals/TOTP, weakening ownership or exposing secrets. Keep public
issues free of private deployment data. Follow [CONTRIBUTING](../CONTRIBUTING.md)
for checks, secret scanning and review expectations.

Production/live acceptance remains separate from a source publication. A public
repository does not activate capabilities or certify deployment. Dated worklogs
and old phase checklists preserve development evidence and may describe behavior
that later commits changed. Confirm current behavior before picking up a task.
