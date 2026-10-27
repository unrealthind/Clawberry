# Phase 5 review — October 21, 2026

Phase 5 is ready for a local review of the implemented flows below. It remains in
progress. Changes are uncommitted on `feat/ui-feedback-oct20`; the branch matched
its fetched GitHub upstream at the start of this session. Earlier uncommitted
chat, Markdown and context-summary work was preserved and included in validation.

## Review path

Use the normal app at `http://127.0.0.1:8080/` and refresh the browser after the
frontend build. `/prototype.html` serves the same app. Real records remain empty
until created; browser-test screenshots use isolated fictional fixtures.

1. **Chat and files:** type a draft and choose Files in the composer. Main chat
   explains the Focus Thread handoff. Create a named thread, upload a file and
   switch between Conversation and Files & evidence. The message draft follows
   the new thread; switching its tabs preserves drafts and preview state.
2. **Source review:** preview Markdown, UTF-8 text/CSV/JSON/log files or a PNG,
   JPEG, GIF or WebP image. Downloads remain authenticated. File previews show
   source identity/checksum. Include text in message explicitly copies a bounded
   source plus provenance into the editable draft; inspect it before sending.
   Uploading alone does not provide file contents to the model. Files over the
   preview/message bounds remain downloadable. SVG/HTML do not render as images.
3. **Thread workspace:** edit its compact summary, inspect linked tasks, record
   a decision and inspect worker findings/sources/archives. Refresh evidence is
   available independently. Unconfirmed uploads block another upload until the
   list is refreshed; decision retries keep the same idempotency identifier.
4. **Tasks and Activity:** open Task history and evidence to inspect recorded
   status changes, worker findings/confidence and sources. Open thread evidence
   from the task. Watch recommendations expose their recorded source evidence,
   investigation/worker identity and reserved budget, separately from actual cost.
5. **Memory:** inspect private/shared provenance and confidence. The sharing
   owner can edit, verify or mark a shared copy stale against its reviewed hash;
   private source memory remains separate. Peer-owned shared facts are inspectable
   without owner management controls. Revocation remains available.
6. **Approvals:** prepare one cross-agent question for an owned task; review the
   exact question before delivery. In the recipient workspace, prepare a scoped
   response; return to the requester for separate exact response approval. The
   exchange closes after delivery. An approved question can be revoked while
   waiting. This human workflow enables no autonomous peer capability.
7. **Rich cards:** completed assistant replies may contain a closed version-1
   info, task, recommendation, security-event or owned-thread-reference card.
   Ordinary text remains Markdown. Invalid/unknown card blocks retain readable
   code fallback; user messages never become interactive cards. A thread action
   checks the current owner's loaded thread list. Cards grant no authority.
8. **Recovery and phone:** stop a reply, refresh history before resending and
   use the header Refresh control for partial outages. Test the navigation drawer,
   thread tabs with arrow keys, downloads and dedicated TOTP review on a phone.
   The context panel excludes expired reviews, includes pending peer reviews and
   shows the configured daily budget without inventing daily usage totals.

The backend binary was rebuilt. A running older backend needs a normal local
restart before it advertises the new card vocabulary to the model. The browser
already reads the rebuilt frontend assets. No live provider turn was required for
this review increment, and no runtime configuration or capability was enabled.

## Remaining Phase 5 work

- Native files in the main conversation and multimodal model image interpretation:
  files currently stay in a Focus Thread, images are human previews, and the
  provider conversation contract remains text-only. A proper implementation needs
  durable attachment/message linkage and bounded provider input, not a hidden
  filename or fabricated model interpretation.
- Integrated pending execution/Guard-exception review cards: existing secure
  backend contracts remain available, but the current Approvals UI covers
  write/TOTP sessions and peer exchanges. A durable human review queue is still
  needed for execution requests; no generic approve-anything UI was added.
- Task and Focus Thread pages now use bounded owner-scoped backend contracts and
  explicit Load more controls, including peer-request task selection. Selected
  older threads remain accessible after refresh. Legacy/model list operations
  retain their 50-row cap. The earlier claim that human list operations returned
  all rows was incorrect; see the connection audit follow-up.
- User visual review and any resulting revisions. Full live n8n/TOTP/Guard,
  production activation and deployment remain Phase 7 work deferred by User.

## Verification

Frontend production build, 18 unit checks, Rust formatting, strict Rust 1.92
all-targets Clippy, the full locked/offline all-target Rust suite and a debug
binary build pass. Browser regression results are recorded in the worklog after
the final run. Browser screenshots/traces are under ignored `app/test-results/`.
No schema migration, live external write, automatic watch activation, credential
change or cross-agent model grant was performed.

Browser coverage: the complete final app run passed 84 cases with one failed upload
case and one intentional skip. The upload test was corrected to wait until initial
loading enables the input; both desktop and phone reruns pass. All 85 applicable
unique cases are therefore covered successfully, with no unresolved test failure.
The corrected upload rerun is retained in `.runtime/phase5-evidence/upload-rerun/`;
full-run screenshots/traces remain in `app/test-results/`.
