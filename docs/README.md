# Documentation index

Reviewed October 22, 2026 (America/Edmonton). Start with the
[project README](../README.md) and [contributor starting points](contributor-starting-points.md)
for the source release, local setup and ways to help finish the project.

## Current state

The project is an early-stage assistant platform with locally tested runtime and
app foundations. Production/live deployment acceptance remains unfinished and
separate from source publication. The [publication review](publication-review.md)
records the clean-history Clawberry plan; the original repository stays private.

The [eight-phase plan](eight-phase-plan.md), [detailed ledger](p06-p20-progress.md),
[phase tracker](../phases.md) and [worklogs](../worklogs.md) preserve implementation
checkpoints. Older "local-only", branch, credential-enrollment and test-count
claims describe their dates, not the current checkout. Confirm current behavior
and final-head CI rather than treating historical checkboxes as release evidence.

## Development and features

- [Rocky development](rocky-development.md): pinned tools, wrapper commands and repository-local Chromium on the Proxmox development VM.
- [Baseline](baseline.md), [permanent agents](permanent-agents.md) and [core work foundation](core-foundation.md): isolated identity, tasks, threads, restricted workers, archives and routing.
- [Controlled execution](execution-foundation.md) and [runtime contracts](p06-p12-runtime.md): policy, Guard, approvals, secrets, rootless Podman and reviewed n8n transport.
- [Connected operations](connected-operations.md): workflow allowlists, canonical manifest hashes, single/bounded TOTP grants, audit correlation, uncertainty and upgrades.
- [Source-aware foundation](source-aware-foundation.md): memory expiry/shared management, bounded watches, notification evidence and exact peer review.
- [UI workspace](ui-workspace.md), [app setup](../app/README.md), [UI direction](../UI_ADDENDUM.md) and [branding](branding.md): current chat-first application, live versus explicit sample behavior and remaining app work.

## Security and release

- [Product requirements](../AGENTS.md), [architecture](../ARCHITECTURE.md) and [security policy](../SECURITY.md): identities, human control and immutable security boundaries.
- [Deployment acceptance](deployment-acceptance.md): deferred target/service/live Guard/n8n/TOTP gates; do not execute activation steps without User's authorization.
- [Merge/release readiness](merge-readiness.md): consolidation history and outstanding final-head CI/release evidence.
- [Contributor guide](../CONTRIBUTING.md) and [PR workflow](pr-workflow.md): validation, review and rollback discipline.

No production schedules, model peer grants, execution capabilities, external writes, credential enrollment or always-on services were enabled by local acceptance. The production target remains to be confirmed; the current Rocky VM is the development machine. Preserve complete stopped-state backups and matching binaries for upgrades/rollback.

## Preserved records

The [October 16 checkpoint](coding-checkpoint-2026-10-16.md), dated worklogs, original upstream attribution and [upstream agent guide](../UPSTREAM_AGENTS.md) remain provenance, not current delivery instructions. Janet/Harley identity files and avatar-generation records are runtime/asset sources rather than release trackers; do not add rolling phase banners to them.
