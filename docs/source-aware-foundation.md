# Phase 4 — source-aware local foundation

This is Phase 4 of the current eight-phase plan, not the older sandbox phase or detailed P04. It covers owned semantic memory, bounded proactive investigations and explicitly reviewed peer exchange on the development runtime. It does not authorize production deployment or live integration activation.

## Memory and provenance

Janet and Harley retain separate private databases and owner-selected retrieval. Facts carry category, confidence, source type/reference, sensitivity, visibility, creation/verification timestamps and optional expiry. Only ordinary, nonstale, unexpired facts with confidence at least 0.5 enter relevant context, with at most six private and six explicitly shared facts. Transcripts remain archives rather than automatic memory.

Worker candidates must reference supplied evidence. Promotion now uses the same input validation as human-managed memory, including credential/token category denial and explicit sensitive-category classification. Ordinary selective promotion is audited and single-use. These metadata checks are not a universal secret or sensitive-content detector; never use semantic memory as a credential store.

Owned expiry review runs before relevant retrieval and on watch ticks, even when investigations are disabled. It marks expired facts stale, records one audit event per transition and retains content for User's review. It neither deletes memories nor silently extends expiry. Verification of an expired fact requires an explicit expiry/content edit first. Verification records human attestation, not an independently checked external-source claim.

## Explicit shared copies

Sharing requires the exact reviewed private-memory hash and creates a separate copy in `shared.db`. Private edits never silently change shared content. The human management operation vocabulary now includes:

- `shared.save(id, reviewed_hash, fact)` — edit the current shared copy, invalidate verification and preserve its private-source linkage.
- `shared.verify(id, reviewed_hash)` — attest to an unexpired copy and clear staleness.
- `shared.stale(id, reviewed_hash)` — exclude the copy from model retrieval pending review.
- Existing `memory.unshare(id)` — revoke the shared copy without deleting the private fact.

Each update is transactional, owner-bound, exact-hash checked and audited. Changed hashes require fresh review; sensitive or credential categories cannot become shared facts. These operations are human-only, even if a model tries to name them. Shared editing controls belong to the Phase 5 app work; the authenticated backend contract is available now.

## Read-only proactive monitoring

Each owner has independent opt-in schedules, explicit UTC quiet hours, daily run limits and reserved investigation budgets. Claims are transactional across hosts. Investigations consume bounded owned due/stale task evidence and failed-read workflow metadata. Their temporary workers receive no tools, credentials, peer memory or write authority. Recommendations follow investigation; failures retain an attention notification and archive rather than claiming success.

Completion/recovery now runs before quiet-hour admission checks. A previously admitted investigation cannot remain stuck just because its schedule was disabled or quiet hours began. Completion and notifications remain idempotent after reopening. Notifications expose their `run_id`, `worker_id`, reserved `budget_microusd` and structured source `evidence` alongside the owned task, allowing recommendations to be traced back to their investigation. Reserved budget is not represented as actual model cost; the model usage ledger remains authoritative for cost.

Schedules stay disabled by default. Local observation is not live mailbox/calendar polling. Broader external-source monitoring and live provider acceptance remain gated; this work does not enable them or install an always-on service.

## Narrow peer exchange

The human management path supports request → exact question approval → one recipient response → exact response approval → closed scope. Pending questions are not delivered to the recipient, pending answers are hidden from the requester, and expiry/revocation prevents new delivery. No operation reads the other owner's private database.

New review hashes include exchange ID and expiry as well as requester, recipient, task, payload and question/response domain. Identical questions in separate requests cannot reuse a review hash. Approval and response admission recompute the current binding inside their transaction. Legacy or changed bindings cannot authorize new delivery: recreate old pending/approved requests and review them afresh. Existing requests can still be revoked or expire; none is auto-approved or extended. History remains auditable after closure; closure cannot retract information already delivered.

Model peer capabilities remain ungranted. Authenticated human review is not permission for autonomous Janet/Harley communication, shared private-database access or external writes. Full conversational/card integration belongs to Phase 5 and live activation to Phase 7.

## Reproduce local checks

```sh
sh dev/rocky-run.sh cargo test --locked --offline --test phase4_foundation
sh dev/rocky-run.sh cargo test --locked --offline --lib assistants::
sh dev/rocky-run.sh cargo clippy --locked --offline --all-targets -- -D warnings
sh dev/rocky-run.sh cargo build --locked --offline
python3 scripts/test_assistant_smoke.py
python3 scripts/test_assistant_smoke.py --local-ui
```

The smoke checks use temporary local fixture state and a loopback model server, not real credentials or external writes. They cover authenticated shared edits/verification/staleness/revocation, the two-stage peer exchange and a fixture-only enabled watch that investigates an overdue task, archives its tool-free worker, emits an evidence-linked recommendation, denies peer access and is disabled afterward. Existing daemon, archive and complete stopped-state backup/restore checks also verify the closed exchange and watch notification survive restoration. Logs belong under ignored `.runtime/phase4-evidence/`. Follow the existing matching complete stopped-state backup procedure before upgrading; Phase 4 adds no schema version or dependencies.
