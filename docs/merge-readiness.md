# Merge history and release readiness

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

PR #1 and the consolidated feature work are already merged into main; redundant feature branches were removed. The October 18 decision and original checkboxes below are historical, not instructions to recreate those branches or hold main unchanged. Current Phase 0–4 changes remain local-only, with acceptance in [the eight-phase plan](eight-phase-plan.md). Fresh final-head CI is still required when publishing; local tests do not imply remote CI has run.

Production readiness is separate from consolidation. The live gates in [deployment acceptance](deployment-acceptance.md) remain deferred; enabling writes or resuming deployment requires User's explicit authorization. A checkbox is evidence-based: code existing is not sufficient.

## October 18 delivery decision

User explicitly chose to **keep the PR in draft until all deployment gates pass**. Preparing commits, updating the feature branch/PR and running CI is authorized; merging into main, enabling external writes or changing deployment security is not authorized by this preparation. Existing unfinished capabilities remain disabled. Main was fetched and checked at `6697f6b80a8e73140ec47cb5689c27ae6e65f73f`; the checked UI branch includes the 84 foundation commits plus the original prototype.

Local verification uses Homebrew Rust 1.99 and Node 24.16.0. Full locked/offline all-target Rust tests pass (2,434 executions; five host-dependent tests ignored), as do tiny and standard suites. Production frontend build and 13 unit/API tests pass. Local Clippy reports newly introduced Rust 1.99 lints across inherited code; the required Rust 1.92 CI remains authoritative and must pass at the final PR head. Desktop/phone browser suite passes: 37 checks and one intentional desktop skip. The locked/offline release build passes. Frontend Prettier, Rust formatting, whitespace and deployment-script shell syntax checks pass; ShellCheck is not installed locally.

The previous security job failed before scanning because cargo-audit 0.22.2 rejects `--locked`. The workflow now pins that release and audits the committed `Cargo.lock` using `--file`; no advisory is suppressed. Final-head security CI is still required.

The consolidated workspace/CI/docs commits were published to both feature branches and [PR #1](https://github.com/unrealthind/Clawberry/pull/1) was verified as draft, with main unchanged. Fresh final-head CI is running; frontend dependency audit passed at inspection. A portable browser-artifact path follow-up keeps Linux CI independent of Mac filesystem paths.

## Automated repository gates

- [ ] Required CI passes on the final branch head with Rust 1.92.
- [x] `cargo fmt --all -- --check` (local Rust 1.99; final pinned CI still required).
- [ ] `cargo clippy --locked --all-targets -- -D warnings`.
- [x] Full, tiny and standard locked test suites pass locally (2,434 / 1,714 / 1,714 executions; five ignored host tests per suite).
- [x] Locked/offline release build passes locally (Rust 1.99).
- [x] Frontend production build, 13 unit/API tests and formatting pass.
- [ ] Rust security audit and cargo-deny supply-chain checks pass.
- [x] Frontend `npm audit --audit-level=high` passes locally: zero vulnerabilities.
- [ ] Deployment scripts pass `sh -n` and ShellCheck.
- [ ] Sandbox image builds from the pinned base image and contains reviewed tools.
- [x] Browser smoke tests pass at desktop and phone viewports: 37 passed; one intentional desktop skip.
- [ ] Workflow sanity/actionlint passes.

## Rocky Linux 10 deployment gates

Run on the intended unprivileged RaaviClaw service account with SELinux enforcing.

- [ ] `deploy/setup-execution-host.sh` completes without disabling SELinux.
- [ ] Rootless Podman works with subordinate UID/GID mappings.
- [ ] Reviewed sandbox image is built locally and referenced by immutable digest.
- [ ] `deploy/verify-rocky10.sh` passes.
- [ ] All ignored `tests/sandbox_acceptance.rs` tests pass against the reviewed image.
- [ ] Orphaned RaaviClaw-labeled containers are removed after an unclean runtime stop while live-owner containers are preserved.
- [ ] User systemd service starts on boot/login as designed, receives the intended config directory/host/port, and stops cleanly on SIGTERM.
- [ ] Service restart recovers state without replaying uncertain writes.
- [ ] SELinux denials are reviewed; no broad permissive mode or blanket allow policy is introduced.
- [ ] Idle/peak RSS is measured on the 4-vCPU/6-GB VM and sandbox cgroup limits are observed.
- [ ] State backup/restore is rehearsed for the complete stopped schema-v9 state tree.

## Guard and execution gates

No Guard category is suppressed merely to make acceptance pass.

- [ ] Live Guard SAFE path passes an explicitly allowed benign request.
- [ ] Live Guard BLOCK path remains blocked pending deep review/human flow.
- [ ] Guard unavailable/malformed/timeout path fails closed.
- [ ] Safer-alternative path preserves agent/task/capability/operation/resource, receives a fresh action UUID, and re-enters Guard.
- [ ] Exact human exception is single-use, hash/request bound and cannot override immutable/root-policy denial.
- [ ] Existing S14 classification on bounded read is diagnosed and documented rather than bypassed.
- [ ] Credentials and Guard provider errors do not appear in model context, chat, artifacts, or audit detail.

## n8n / TOTP gates

n8n remains on TrueNAS SCALE. External writes stay disabled until all write gates pass.

- [ ] Named read-only contract works through the deployed Rocky service with exact field/resource validation and response projection.
- [ ] n8n read errors/timeouts remain non-retrying and auditable.
- [ ] TOTP authorization uses only the dedicated authenticated endpoint; code never enters model/chat/operation logs.
- [ ] Single-action write binds exact agent, task, broker identity, workflow, manifest/hash, payload and expiry.
- [ ] Bounded session budgets are atomic under concurrent actions.
- [ ] Duplicate/replayed action IDs are rejected.
- [ ] Equivalent unresolved writes remain blocked across a new task/session.
- [ ] Process/runtime restart converts expired reserved writes to uncertain and does not refund budget.
- [ ] Ambiguous write result requires final read-side reconciliation before any equivalent retry.
- [ ] Gateway status evidence binds exact action ID and payload hash and reports a terminal state.
- [ ] Session end revokes local authorization/token before or independently of remote cleanup.
- [ ] Cross-workflow replay is prevented even where workflows share a TOTP secret.
- [ ] At least one controlled live write acceptance is completed against a non-destructive test target before production writes are enabled.

## Browser and product gates

- [ ] Built app loads from the Rust gateway same origin.
- [ ] Pairing/token entry remains in memory only and clears on disconnect/reload.
- [ ] Janet/Harley ownership boundaries are exercised in browser flows.
- [ ] SSE delta, completion, abort/Stop, reconnect/history refresh and error states work.
- [ ] TOTP review UI does not persist or expose codes.
- [ ] Core chat/threads/tasks/activity/approvals/memory flows work at desktop and phone viewport.
- [ ] Invalid rich-card payloads/actions remain non-executable.
- [ ] Main deployment is reachable only through an administrator-approved HTTPS reverse proxy or SSH tunnel; direct public gateway exposure is not used.

## Final merge gate

- [ ] `docs/p06-p20-progress.md`, `phases.md`, and `worklogs.md` reflect final evidence without overstating incomplete phases.
- [ ] Draft PR final head has green required CI and security checks.
- [ ] No temporary write-enabled maintenance workflows remain in `.github/workflows`.
- [ ] PR receives a final diff/security review after the last code change.
- [ ] User explicitly approves merging the PR.


CI update at workspace head `202ac39`: the required Rust 1.92/Clippy/full-tiny-standard/build gate, frontend build/tests/format, desktop/phone browser checks, deployment-static/image checks, actionlint, npm audit and cargo-deny passed. The corrected RustSec audit ran and found six advisories in locked quinn-proto 0.11.13; all require at most 0.11.18. A targeted lockfile patch to 0.11.18 (and its required transitive dependencies) passes the full, tiny and standard locked/offline local Rust suites and release build, without suppressing advisories. Final patched-head CI remains required. PR #1 was draft at that checkpoint; consolidation is now complete, while deployment gates remain open.

October 19: steps 1–3 local product delivery now include actual Claude route verification, default live UI and schema-v8 versioned backend settings. Final check evidence is in the product plan/worklog. Production web credential enrollment and all unchanged VM/Guard/n8n/write acceptance gates remain open; PR stays draft.

## October 19 steps 4–5 follow-up

Step 4 is complete locally: authenticated thread file transfer, schema-v9 idempotent human decisions and structured worker/archive inspection. Local full Rust tests: 2,445 passed, five existing host sandbox tests ignored. Frontend build/14 unit checks and 43 browser checks pass (one intentional skip). Both gateway authentication modes pass the full fixture-state backup/restore smoke. A new two-connection write-budget race admits exactly one reservation. The Linux systemd unit now has real line breaks, covered by a regression.

The previously published `65d76d5` Rust 1.92 CI failed the independent GatewayConfig boolean lint; the narrow compatibility allowance is now corrected without changing the Clippy gate. Its security audit, cargo-deny, npm audit, actionlint, frontend, browser and deployment-static/image jobs passed. These results do not establish new-head CI.

Step 5 remains open. No confirmed target SSH alias/service config was supplied in this session, and no new live Guard/n8n/TOTP write or VM acceptance was performed. Do not check off target-host gates from fixture evidence. See [the acceptance procedure](deployment-acceptance.md). Production credential enrollment remains unfinished. The draft/main-unchanged restriction applied at that checkpoint and was superseded by the authorized consolidation.

User subsequently chose to put step 5 on hold as too advanced for the current stage. Deployment acceptance is deferred until explicitly resumed. Keep the completed local step 4 delivery; PR remains draft and main unchanged.


October 19 UI consolidation: the legacy shell/CSS are removed. Both frontend URLs serve the current workspace; approvals, memory and watch controls are integrated. Local build/14 unit checks and 57 desktop/phone browser checks pass (one intentional skip), including dedicated TOTP transport and uncertain-authorization handling with fixtures. No live write or target-host acceptance was performed. Deployment remains deferred, final-head CI is required, PR stays draft and main unchanged.

October 19 chat polish: production build, 14 unit/API tests and 67 desktop/phone browser checks pass (one intentional desktop skip). Checks cover profile dispatch, composer growth, desktop/touch input, focus restoration, Stop/uncertain-response recovery, copy success/failure and long-history scrolling. Backend contracts are unchanged; this does not satisfy or resume live deployment gates.
