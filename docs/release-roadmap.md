# Release roadmap

Reviewed October 20, 2026 (America/Edmonton), after fetching origin and comparing main with its upstream. Checked HEAD/origin/main are `c62e27b` with no divergence. Phase 0–4 changes remain local-only, not published on GitHub. See [the current eight-phase plan](eight-phase-plan.md).

## Historical branch baseline and consolidation

- `main`: `6697f6b`, planning and upstream TinyClaw baseline.
- `feat/multi-agent-foundation` and `feat/ui-workspace`: both at `1f97292`, identical trees, 98 commits ahead of main.
- User subsequently authorized merging all work to main and removing redundant branches. Main was fast-forwarded to `1f97292` and pushed; both feature branches were deleted only after verifying their commits were included. All applicable reported PR checks passed. GitHub's PR merge endpoint reported inconsistent head metadata, so consolidation used a normal Git fast-forward with no force push.
- Active implementation and acceptance records now live on main in `docs/product-plan.md`, `docs/p06-p20-progress.md`, `docs/merge-readiness.md`, `phases.md`, and `worklogs.md`.
- Feature-branch records report P00–P05 complete locally, including permanent agents, tasks/threads, restricted workers, archives, and model routing. Security, integrations, memory and app work are substantial but retain open acceptance gates.
- Earlier recorded tests remain historical evidence. Fresh Rocky VM revalidation and repeatable development commands are recorded in `docs/baseline.md` and `docs/rocky-development.md`.

The initial main-only assessment understated implementation progress. Do not restart Phase 1 or replace existing work with a new foundation. The branch hashes above record the pre-consolidation baseline.

## Development environment

User confirmed this Rocky VM on Proxmox is the development machine. This does not authorize production deployment or resolve the conflicting older deployment-target records. Deployment acceptance remains deferred until explicitly resumed.

## Remaining delivery sequence — current numbering

| Phase | Scope | Acceptance |
| --- | --- | --- |
| 0–4 — Complete locally | Baseline, isolated agent/work foundation, controlled execution, connected operations and source-aware memory/watch/peer foundations | Full/tiny/standard 2,494/1,768/1,768; strict Clippy; debug/release recovery; 14 frontend and 67 browser checks, one intentional skip; host opt-ins retain separate Phase 2 evidence |
| 5 — Complete app flows (next) | Main-chat attachments/images, Files navigation, evidence panels, shared editing controls, conversational peer review and rich cards | Connected flows handle empty/loading/error/recovery states; desktop and phone checks pass |
| 6 — Daily-use beta preparation | Recovery/upgrade tooling, complete-state backup/restore, budgets, maintenance and retention | Representative daily workflows pass; limitations are explicit |
| 7 — Production and release (deferred) | Only after User resumes deployment: confirm target, service, live Guard/n8n/TOTP, replay/atomicity/reconciliation and final-head CI | Reviewed activation gates pass before enabling production capabilities or writes; merging alone is not release acceptance |

The chat app now exists on main. Preserve it and extend its current contracts. User's explicit consolidation request supersedes earlier instructions to keep main unchanged and PR #1 draft. It does not resume deployment acceptance or authorize enabling external writes. Earlier draft/branch instructions elsewhere remain historical.

Desktop packaging follows the usable web beta. Voice, wake word and home/phone integrations remain post-v1.
