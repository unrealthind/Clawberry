# Frontend/backend connection audit

Reviewed October 21, 2026 (America/Edmonton), against the current uncommitted
working tree on `feat/ui-feedback-oct20`. Earlier local work was preserved. These
results describe the development environment, not a production release signoff.

The tested UI flows connect correctly to the Rust backend and preserve the tested
authentication, owner and approval boundaries. The audit found and repaired the
issues below. Live external-write/Guard deployment acceptance remains open.

## Findings and repairs

Severity here describes the observed local failure; it is not a formal risk score.

| Finding | Severity | Repair and evidence |
| --- | --- | --- |
| JSON requests and file transfers could remain pending indefinitely | Medium reliability | Deadlines include response consumption: 45 seconds for ordinary requests/transfers, ten minutes for chat. Cancellation propagates; no automatic mutation replay. Unit tests and desktop/phone stalled-task tests preserve drafts and block blind resubmission. |
| A valid SSE completion still waited for EOF | Medium reliability | Completion is the terminal server acknowledgement; the reader now returns and cancels the remaining stream. A never-closing stream reproducer passes. |
| A complete oversized SSE event bypassed the size check; many deltas could accumulate indefinitely | Medium robustness | Enforce the event limit before JSON parsing, and cap aggregate streamed text before rendering further deltas. Oversized terminal and cumulative-delta unit regressions cover both cases. |
| Malformed JSON responses could reach UI renderers | Medium reliability | Runtime validation for lists/pages, nested task evidence, write-session budgets/actions, complete global/agent settings, runtime credential status, and task/thread creation and TOTP authorization acknowledgements. Errors use recoverable states and never replay mutations. A failed session refresh clears previously displayed status. Additive metadata is accepted. |
| Older tasks and threads disappeared after the first 50 records | Medium functionality | New owner-scoped `task.page` / `thread.page` operations use bounded database queries and cursors; UI and peer-request task selection expose Load more. `thread.get` preserves an older selected thread after refresh. Legacy/model list operations retain their 50-record limit. |
| Expired memories could not be explicitly updated in the UI | Medium functionality | Private/shared editors now expose RFC 3339 expiry and explicit removal. Saving clears verification; backend checks expiry and owner/exact-hash boundaries. Verification remains a separate action. |
| A dangling runtime-config symlink was rejected only after writing through it | Medium local filesystem safety | Check before creating the configuration file. The regression proves the symlink target remains absent. This is not a claim of protection from a hostile host administrator. |
| npm/Vite inherited other providers' environment credentials | Low defense in depth | GUI subprocesses now receive an explicit environment allowlist. The gateway retains its provider environment. Reproduced with dummy OpenAI/generic provider variables; five launcher tests cover the final behavior. No browser-bundle leak was observed. |
| Some private/error responses omitted server-side cache controls | Low privacy hardening | Gateway-wide `Cache-Control: no-store`, including failed authentication and pairing responses. The real HTTP audit verifies private reads and authorization failures. |
| Reusing a built GUI across paired/local login modes could display the wrong connection screen | Medium startup reliability | Build HTML carries nonsecret login-mode metadata. `--no-build` rejects a mismatched or legacy build with a full-rebuild instruction. Tests cover correct and mismatched modes. |

Vite filesystem access is also explicitly limited to the app directory, with
credential/runtime paths denied. The live Vite test checks traversal/outside-app
paths and a temporary `.env.audit-*` file containing a public sentinel. A missing
URL may return Vite's HTML application fallback; that is distinguished from serving
private file bytes. No actual credential content is printed or saved by the audit.

## Real connection coverage

`scripts/test_ui_backend.py` starts an isolated Rust gateway, Vite frontend and
loopback model fixture. `app/e2e/ui-backend-contract.mjs` uses a real browser and
real API handlers, rather than intercepting them with mocked JSON responses.
Temporary fixture state is removed afterward. The existing development server is
not stopped or replaced.

| Flow | Observed result |
| --- | --- |
| Paired login through Vite, including localhost | Anonymous/invalid keys denied; authenticated UI loads; no private state before login. |
| Model profile and chat | Fast selection reaches the real model router; completed reply is present in durable thread history. The provider is a local fixture, not a paid/live service. |
| Lost response after commit | Test lets the backend commit, then deliberately drops the browser response. Draft stays visible, sending is blocked until refresh, history reveals one committed turn, and no replay occurs. |
| Tasks and Focus Threads | UI creation/details match stored records; task evidence and thread links work. |
| Files and decisions | Actual upload, owner-scoped download, preview/text inclusion, durable decision and reconnect persistence work. Peer-owned file/task/thread reads are denied. |
| Settings | A valid preference save persists; the stale revision returns 409 without overwriting it. |
| Memory | Exact shared-copy edit persists while the private source remains unchanged. Stale hashes and peer-owner management are rejected. |
| Cross-agent exchange | Separate exact question/response review, recipient preparation and final closure all work through the real backend. This grants no autonomous peer capability. |
| Authorization transport | Missing-review TOTP fails closed; code stays on the dedicated endpoint. No external action is dispatched. Full write-session UI uses separate fixture coverage, not a deployed n8n write. |
| Credentials/session lifecycle | Fixture bearer/code do not enter browser storage, URLs or model input. Disconnect clears the active session; reconnect retrieves saved records. |

The broader paired and direct-loopback smoke suites also verify bounded/restricted
workers, budgets, failure handling, SSE/internal-action suppression, source-aware
memory, approved peer boundaries, watch investigation evidence, runtime restart and
complete stopped fixture-state backup/restore with matching hashes and SQLite
integrity. Direct-loopback mode rejects forged browser origins/hosts and forwarding
headers. Those schedules and workers exist only in temporary fixture state.

## Remaining limitations

- **LAN HTTP is not encrypted.** Pairing authenticates possession of a key; it does
  not prevent a network observer from capturing that key, TOTP or an entered API
  credential. Keep this mode on a trusted development network. Untrusted-network
  and production access need HTTPS or a protected tunnel; that acceptance was not
  performed here.
- Live HF Guard, real n8n/TOTP writes, gateway replay/concurrency/reconciliation and
  always-on deployment remain separate release gates. Local fixtures do not prove
  those external services are configured or transactionally safe.
- Browser phone coverage is Chromium device emulation, not a physical phone or
  Safari/iOS certification.
- Task/thread pages are bounded to 50 rows by default (maximum 200). The UI
  loads more explicitly. Automated acceptance covers 125 owned records, peer isolation and
  complete cursor traversal; this does not certify multi-million-row capacity.
  Legacy/model list operations remain capped at 50; `AgentHost::tasks/threads`
  return all rows for internal callers. The earlier report confused these layers.
- Rendered settings/status/evidence and important creation/authorization replies
  now have runtime validation. Future protocol extensions and every unused
  mutation acknowledgement are not exhaustively schema-validated. The backend
  remains authoritative for policy, payload validation and approval enforcement.
- Main-chat native attachments, model image interpretation and a pending
  execution/Guard-exception review queue remain feature gaps from Phase 5. These
  are separate feature work; none grants extra authority.
- The frontend still emits a nonfatal bundle-size warning. Startup metadata guards
  login-mode reuse; switching modes requires a normal full rebuild.

## Reproduction and evidence

Use the repository's Rocky wrapper and installed Playwright browser cache:

```sh
sh dev/rocky-run.sh python3 scripts/test_dev_local.py
PLAYWRIGHT_BROWSERS_PATH=/tmp/raaviclaw-playwright sh dev/rocky-run.sh python3 scripts/test_ui_backend.py
sh dev/rocky-run.sh python3 scripts/test_assistant_smoke.py
sh dev/rocky-run.sh python3 scripts/test_assistant_smoke.py --local-ui
sh dev/rocky-run.sh cargo test --locked --offline --all-targets -q
```

Frontend checks run from `app`: `npm run build`, `npm test`, and
`PLAYWRIGHT_BROWSERS_PATH=/tmp/raaviclaw-playwright npx playwright test`.
Rust formatting/strict all-targets Clippy and frontend formatting also pass.
Rust results are retained under ignored `.runtime/connection-audit/`; browser
screenshots/traces use ignored `app/test-results/`. No user credentials, model
context or real private source content is added to the report or test artifacts.

All repairs remain local and uncommitted. Restart the local development command to
activate the rebuilt backend; the existing running process was not interrupted.

Original audit results: **89 browser checks passed, one intentional skip; 25 frontend
unit checks passed; five launcher checks passed; 2,499 Rust tests passed with eight
existing opt-in host execution counts ignored.** Both backend smoke modes and the
real browser/proxy/backend audit pass. There is no unresolved failure in these
executed checks. This does not close the limitations listed above.

## Audit follow-up closure

The follow-up closes the local response-validation, task/thread paging and memory
expiry-edit gaps. Pages use database limits and owner-scoped cursors; older selected
threads resolve through an owner-checked lookup. Task summaries and calendar views
explicitly disclose when older tasks remain unloaded, so a partial page does not
claim that all deadlines or active work have been checked.

The pagination regression traverses 125 tasks and 125 threads without duplicates,
excludes peer rows and rejects invalid limits. Browser tests cover loading older
rows and reopening an older selected thread. The new test caught a mutex lifetime
issue during implementation; it was corrected before delivery. Malformed nested
evidence, settings and session budgets produce recoverable errors without
crashing the shell or fabricating an active grant.

The real backend fixture rejects verification of an expired fact, then accepts an
explicit UI expiry edit/removal and separate verification. A shared-copy expiry
edit leaves the private source unchanged and clears verification.

HTTPS deployment, live external services, physical-device certification and the
separate Phase 5 feature gaps above remain open; they are not covered by this local
connection-work closure. No live service was restarted and no external writes
were enabled. All changes remain local and uncommitted.

Final follow-up validation: **97 browser checks passed, one intentional skip;
27 frontend unit checks passed; 2,501 Rust checks passed, eight existing opt-in
ignores; zero failures.** Frontend build, strict all-target Clippy, formatting,
and the actual paired browser/proxy/backend fixture also passed. Final browser
checks ran against a fixed build; an earlier overlapping rebuild caused one blank
page in the test harness and was discarded. Follow-up logs are retained at
`.runtime/connection-audit/followup-browser-tests.log` and
`.runtime/connection-audit/followup-rust-tests.log`.
