# Eight-phase delivery plan

Updated October 20, 2026 (America/Edmonton). This numbering follows User's current eight-phase plan. The older P00–P25 identifiers in `phases.md` remain useful implementation references; they are not interchangeable with the phase numbers here.

| Phase | Scope | Status |
| --- | --- | --- |
| 0 | Rocky development baseline: pinned tools, inventory, builds/tests and recovery | Complete locally; see `baseline.md` |
| 1 | Agent/work foundation: P01–P05, identity isolation, tasks/threads, restricted workers, archives and model routing | Complete locally; freshly verified on Rocky |
| 2 | Security/controlled execution: P06–P09, hard denials, scoped approvals, Guard/deep review, secrets and rootless sandbox/package operations | Complete for local development acceptance; real rootless checks pass with SELinux enforcing; live-provider and production gates remain separate |
| 3 | Connected operations: n8n reads, TOTP writes and execution reconciliation | Complete for local client/runtime acceptance; live n8n/TOTP and external-write activation remain gated |
| 4 | Source-aware memory, proactive monitoring and approved cross-agent exchange | Complete for local foundation acceptance; schedules and model peer capabilities remain disabled by default; live activation remains gated |
| 5 | Complete chat app flows, attachments, Files navigation, evidence panels and rich cards | In progress; October 21 review adds thread tabs, file/image previews, text-to-draft handoff, rich cards, task/watch evidence, shared-memory controls and human peer exchange flows. See `phase5-review.md` for remaining gaps |
| 6 | Daily-use beta, budgets, maintenance, retention, recovery and upgrade tooling | Partially verified |
| 7 | Production target, always-on service and live integration/release acceptance | Deferred by User |

## Phase 1 acceptance

The existing implementation already supplies separate Janet/Harley identities, workspaces, private memory and conversation histories; owned durable tasks and Focus Threads; context/budget-scoped temporary model workers; cancellation/recovery; inspectable archives and selective memory candidates; and configurable fast/normal/deep routing. No new duplicate agent registry is needed.

Fresh Rocky evidence: `assistant_isolation` passes 11 integration tests and `work_foundation` passes six, following Phase 0's full/tiny/standard and paired/loopback daemon recovery checks. Worker tools and credentials remain empty by default; completion of this foundation does not grant shell or secret access to workers.

## Phase 2 local hardening

Guard transport now requires exactly one normally completed verdict. Truncation, filtering, malformed/oversized bodies, redirects, HTTP failures and timeouts cannot authorize execution. The production endpoint/model are unchanged; test transport addresses are private fixture implementation details, not model configuration.

Execution identities must be canonical UUIDs and scope/payload sizes are bounded before Guard or approval consumption. Safer alternatives retain identity/task/capability/operation/resource scope, use a fresh valid action identity, re-enter Guard and require their own exact human approval. Deep review never grants execution.

Credential redaction additionally covers padded/unpadded standard/URL-safe Base64 and uppercase hex, including nested JSON keys. This covers tested representations, not arbitrary transformations of a secret. Existing rotation, expiry, revocation and per-owner/capability checks remain enforced.

Sandbox output remains within its byte limit after invalid UTF-8 replacement. Rootless user mapping explicitly maps the invoking host user to container UID/GID 1000, and private input mounts request SELinux relabeling. Host execution, network access, image pulling, credentials and broad worker permissions remain unavailable through the sandbox.

Fresh local validation: strict Rust 1.92 all-targets Clippy and formatting pass; final full suite passes 2,457 executions. Tiny/standard previously passed 1,731 each during hardening. Eight execution counts are opt-in in the final full suite: six host acceptance checks and the broker/container check compiled into both unit-test targets. All seven distinct real acceptance checks were separately run and passed on October 20. Debug/release builds and paired/token-free fixture daemon/recovery smoke checks pass; the optimized binary also passed the token-free recovery smoke before the final package-output correction, which was verified in the debug/runtime acceptance path.

## Real-host acceptance — complete locally October 20

User installed the prerequisites and protected default-deny policy through the host terminal. Podman 5.8.2 reports rootless mode and SELinux reports Enforcing. The reviewed local base image is `localhost/raaviclaw-sandbox@sha256:b6007ca91d75d849dadf67de5582e234bd954d4a187ba64fb90139a39755fe85`.

Five real-container integration checks passed: read-only/networkless/privilege isolation with actual CPU/memory/swap/PID limits, image preview, short-video preview, live-owner-preserving orphan cleanup, and offline package search/install/import/removal with unbundled-package rejection. A separate broker/container check proves single-use approval stays spent after reopening; the protected default-deny host-policy check also passes. No labeled sandbox containers remained afterward.

The runner builds a test-only derivative image with a deterministic pure-Python fixture wheel, using a temporary context and `--pull=never --network=none`. It does not add packages to the base/runtime image or enable capabilities. Package acceptance caught and fixed literal escaped newline/JSON output in package search/removal. `.runtime` is now excluded from both Docker and Podman build contexts.

Reproduce setup manually from User's terminal if required:

```sh
sudo sh deploy/setup-execution-host.sh
podman info --format '{{.Host.Security.Rootless}}'
podman build -f deploy/Containerfile.sandbox -t localhost/raaviclaw-sandbox:acceptance .
podman image inspect localhost/raaviclaw-sandbox:acceptance --format '{{.RepoDigests}}'
```

The setup script installs prerequisites and a default-deny policy only if absent; it does not enable execution, install a service or overwrite an existing policy. Select the actual reviewed local RepoDigest, then run:

```sh
RAAVICLAW_SANDBOX_IMAGE='localhost/raaviclaw-sandbox@sha256:ACTUAL_REVIEWED_DIGEST' sh dev/phase2-acceptance.sh
```

The runner refuses absent images and non-rootless Podman. It checks real isolation, timeout/output limits, media decoding, offline packages and orphan cleanup; then checks single-use broker admission against a real container and protected default-deny host policy. Its Guard verdict is an explicit local fixture, not live HF acceptance. No fake host shell substitutes for Podman.

Persistent custom package sets and worker execution grants remain unavailable; common utilities are bundled in the reviewed image and custom wheel installs are disposable/offline. Live HF/S14 diagnosis, enabled integration/service acceptance and TOTP writes remain separately gated; this plan does not declare those release gates complete.

## Phase 3 — complete locally October 20

See [connected operations](connected-operations.md) for the reviewed protocol, canonical manifest hashes, per-workflow operation allowlists, read correlation/projection, dedicated TOTP handoff, exact single-action and bounded-session grants, failure cleanup, durable closure evidence and uncertain-write reconciliation.

Final local evidence: strict Rust 1.92 Clippy/formatting; full suite 2,481 passed; tiny/standard each 1,755 passed; eight real-host execution counts remain opt-in and retain Phase 2's separate passing acceptance. Debug/release builds, paired/token-free debug recovery and optimized-binary recovery smoke pass. Frontend build, 14 unit/API tests, formatting and 67 desktop/phone browser checks pass with one intentional skip. Loopback gateway fixtures exercise authorization/dispatch/restart/closure; no live n8n operation or external write was performed.

The separate write ledger now migrates to schema version 1 without restoring budgets or spent authority. Existing write contracts require explicit `allowed_operations`; old bounded-session pending reviews require fresh canonical hashes. Back up the complete stopped state before upgrade and use a matching backup/binary for rollback.

Production enrollment, actual n8n/Guard validation, shared gateway TOTP replay/transactional enforcement and live test writes remain Phase 7 gates. Runtime execution configuration remains absent; no model capabilities, credentials, workflow installations or services were enabled. All Phase 3 changes are local-only until committed/pushed.

## Phase 4 — complete locally October 20

See [source-aware foundation](source-aware-foundation.md) for owned semantic memory, selective validated promotion, automatic audited expiry review, exact-reviewed shared-copy editing/verification/staleness/revocation, independent bounded read-only investigations and narrowly approved peer exchanges.

Peer review now binds exchange identity and expiry as well as the exact payload, owner and task. Legacy or changed bindings fail closed for new delivery and require a fresh request/review; revocation and retained history remain available. Watch completion/recovery runs during quiet hours and after disabling schedules. Notifications retain structured source evidence and their investigation/worker/budget correlation. Memory promotion shares the management validator, including token/credential category denial; expiry review retains facts for human review rather than deleting them or silently extending validity.

Final full/tiny/standard suites pass 2,494/1,768/1,768 test executions; eight real-host execution counts remain opt-in with Phase 2's separate evidence retained. Strict Rust 1.92 Clippy/formatting, debug/release builds, paired/local-UI debug daemon recovery and optimized-binary local-UI recovery pass. Frontend build and 14 unit/API checks pass; desktop/phone browser acceptance passes 67 checks with one intentional skip. Authenticated fixture smoke exercises shared-memory edits, separate exact question/response review, a successful evidence-linked tool-free watch and complete stopped-state restoration of its notification and closed exchange. Logs are under `.runtime/phase4-evidence/`.

No production schedules, peer model grants, credentials, execution capabilities, external writes or services were enabled. Broader external-source monitoring, independent source re-verification, live provider acceptance and production activation remain gated. Shared editing controls and full conversational/card integration belong to Phase 5; the older detailed P13–P15 items retain those broader gates. Work is local-only until committed/pushed.

## Phase 5 — local review increment October 21

See [the review guide](phase5-review.md) for completed app journeys, verification
and the remaining native main-chat attachment/multimodal, execution-review queue
and list-pagination gaps. Phase 5 is not marked complete by this increment. All
changes are local and uncommitted; live activation and Phase 7 remain deferred.
