# Clawberry app foundation

This is an early-stage React/TypeScript/Vite chat workspace using assistant-ui.
Start with the [project README](../README.md) for loopback setup and the
[contributor starting points](../docs/contributor-starting-points.md) for open work.
Production/live acceptance remains separate. Dated notes later in this guide
preserve historical evidence; their test counts and unfinished-feature statements
may have been superseded by subsequent commits.

## Build and run on the LAN

From the repository root:

```sh
sh dev/rocky-run.sh python3 scripts/dev-local.py
```

This builds Rust and the GUI, then runs the paired gateway on loopback port 8080
and the Vite GUI on LAN port 5173. Before starting the servers, LAN mode runs
`sudo firewall-cmd --zone=public --add-port=5173/tcp --timeout=1h`; enter your sudo
password in the terminal if prompted. This is a temporary runtime rule, not a
permanent firewall change. It expires after one hour even if the app is still
running; restart the launcher to renew it. Existing firewall rules are preserved.
Failure stops startup with an error. `--local` does not change the firewall. The terminal prints the LAN URLs and a fresh
**DEVELOPMENT ACCESS KEY**. Paste that key into the GUI's gateway-token login field,
even when using localhost. This is a gateway login key, not a Claude API key.
Ctrl+C stops both child servers. Occupied ports cause an error; stop the previous
dev server first. No firewall or service configuration is changed.

The key is valid for this gateway run and is not stored in a file or browser
storage. Existing paired tokens continue to work. LAN HTTP does not encrypt
traffic; use a trusted LAN. The gateway remains paired and refuses a non-loopback
`--dev-access` bind or configured tunnel. Policy, Guard and TOTP remain enforced.
Only the gateway receives the private Claude environment credentials; npm/Vite
receive no Anthropic credential values.

Use `--no-build` for a quick restart with the same login mode. Switching between LAN and `--local` requires a full build; the launcher rejects a mismatched or older GUI build rather than showing the wrong login screen. Use `--local` for the previous loopback-only,
token-free gateway at `http://127.0.0.1:8080/`, without a LAN Vite server. `--build`
is accepted for compatibility; building is now the default. `--paired` explicitly
selects the default paired LAN mode. UI hot reload runs through Vite; Rust changes
need a restart/rebuild.

## Local development

For local Claude development, copy `.env.example` to `.env.local` at the repository root and set `ANTHROPIC_API_KEY` there (or use a preconfigured shell environment). Never paste the token into chat or a VITE_* variable. The launcher loads only Anthropic credential assignments without executing shell expressions and keeps `.env.local` private and git-ignored.

```bash
sh dev/rocky-run.sh python3 scripts/dev-local.py --local
```

The launcher starts loopback port 8080 with persistent git-ignored `.runtime/` state and schema-v9 persistent state. Verify real Fast/Normal/Deep routing with `python3 scripts/verify-local.py`; it makes three small billable provider requests in a verification Focus Thread. Production credential enrollment will use a secure web UI before deployment acceptance; it remains disabled now.

In another terminal:

```bash
cd app
npm ci --ignore-scripts
npm run dev
```

Open `http://127.0.0.1:5173` or `/prototype.html`. Localhost connects without a token form or Authorization header. `--local-ui` also works with `daemon`, or set `[gateway] local_ui = true`. It refuses non-loopback binding, tunnels and trusted proxy headers; browser Host/Origin/fetch metadata must be local and forwarded requests are rejected. No external-write/TOTP, Guard or agent-boundary checks are changed.

For remote use, omit `--local-ui` and keep pairing enabled. Enter a paired gateway token in the dedicated login form—not in chat. The token is not saved in local/session storage, URL parameters, logs or model messages. Disconnect clears the active token and cached domain data. Already submitted server work can still finish.

The development proxy has a fixed loopback destination, `127.0.0.1:8080`. Do not enter credentials into an arbitrary frontend or configure a model-supplied destination. For a remote VM, use an administrator-configured SSH tunnel or authenticated HTTPS reverse proxy. For production, build the app and set `gateway.app_dir` to its absolute `dist` directory; the gateway serves static assets on the same origin. Keep authenticated HTTPS or a local SSH tunnel.

## Checks

```bash
npm run build
npm test
./node_modules/.bin/prettier --check src e2e playwright.config.ts vite.config.ts tsconfig.json package.json index.html
npm run test:e2e
```

Local build and 14 unit/API tests pass with Node 24.16.0. See [merge-readiness evidence](../docs/merge-readiness.md) for current desktop/phone browser and final-head CI results. These checks do not establish live deployment or external-write acceptance.

## Available and intentionally unavailable

Available: separate Janet/Harley main conversations, Focus Thread create/open/history, thread file upload/download, owned task/activity/worker views, private memory search/edit/delete/verify/stale controls, and human-only exact cross-agent question/response review.

Also available: authenticated SSE chat with Stop (provider deltas when supported, single-result fallback otherwise), paginated main/thread history and private memories, exact human-reviewed shared-memory copies/revocation, and disabled-by-default independent read-only watch schedules/recommendations under Activity.

The remote integration also supplies opt-in guarded shell/package/media operations and dedicated TOTP review/session controls; these require administrator-owned configuration and their outstanding acceptance gates. Main-chat file/image understanding, complete shared-memory management and autonomous external-source watch integrations remain unfinished. No live external-write acceptance is claimed. These limitations are visible in the app; there is no simulated execution or fake TOTP authorization.

Rich cards accept a closed version-1 schema. Arbitrary HTML, executable UI, unknown actions and unsupported versions are rejected. The runtime card-envelope integration is still pending.

## Chat-first workspace

Run `npm run dev:prototype` and open `/prototype.html`. assistant-ui manages separate Janet/Harley conversations and Focus Threads, scripted sample replies, and cancellation. Tasks, calendar deadlines, thread context and fictional worker handoffs share tab-local state. Only explicit `?mode=sample` makes no gateway requests or external writes. Live is the default.

Global app settings open from the User footer or Command/Control + comma; agent setup opens beside Janet or Harley and consolidates identity/memory files, skills, credential previews and boundaries. Shared Claude API/model profiles remain global. Sample model selections are local drafts; live settings persist actual runtime configuration; raw API-key/credential entry remains disabled.

Global Settings → Connection & security → Connect gateway opens an opt-in live workspace with the workspace sidebar, streamed chat, older-history loading, Fast/Normal/Deep requests, server-backed Focus Thread creation/context, task editing, read-only activity and calendar task projections. Activity/file metadata failures do not block chat/tasks. It uses the existing same-origin pairing contract; Approvals and memory open inside the same workspace without changing URL or authentication context. See [workspace scope](../docs/ui-workspace.md) and [detailed preview guide](src/prototype/README.md).

Live global and owner settings now persist through dedicated human-only backend endpoints. See [product plan](../docs/product-plan.md) and [settings API](../docs/core-foundation.md).


## October 19 thread and recovery delivery

Step 4 is complete locally: authenticated thread file upload/download, schema-v9 durable human decisions with idempotent retries, and structured worker results/archive downloads. Local validation: 2,445 Rust tests and 14 frontend checks passed; 43 browser checks passed with one intentional skip; five existing host sandbox Rust tests remain ignored. Both gateway modes passed complete stopped fixture-state backup/restore. The Linux service unit newline bug is fixed, and concurrent write-budget admission is tested across independent connections.

Step 5 is deferred by User; target VM, live Guard/n8n/TOTP and always-on acceptance remain open. Production credential enrollment remains unfinished. Final-head CI is required; At that historical checkpoint PR #1 was draft and main was unchanged; the later authorized consolidation superseded that restriction. See [deployment acceptance](../docs/deployment-acceptance.md).


## Live workspace UI follow-up

App settings now has an explicit footer label and gear icon, with a selected state. Agent setup remains beside each agent. Live pages use consistent controls/cards and restore the intended settings sections, appearance choices, model shortcut and keyboard hints. The header appearance toggle saves through the backend with revision conflict handling. Mobile navigation supports focus containment, Escape and hidden-drawer inertness. Credentials enrollment remains clearly unavailable. Validation: production build, 14 unit/API checks and 45 desktop/phone browser checks pass; one intentional browser skip. Deployment remains deferred.


## One application

`/` and `/prototype.html` load the same current workspace bundle. The legacy `src/main.tsx` management shell and its global `style.css` have been removed. Approvals and Memory are normal sidebar destinations; settings opens approvals through an in-app callback. Watch schedules and recommendation review remain in Activity. Disconnect clears the active session and shows Connect; fictional samples require an explicit sample choice.

External write review retains exact manifests/hashes, dedicated non-caching TOTP submission, uncertain-result blocking, session expiry/budgets/results and End session. Cross-agent review retains exact question/response hashes. Memory retains paginated private/shared reads, filters, edit/verify/stale/delete, exact reviewed sharing and revocation. No external write is dispatched by the migration and no deployment gate is declared complete.

Validation: build and 14 unit/API tests pass; 57 desktop/phone browser checks pass, one intentional skip. New fixture checks exercise both entry URLs, in-app approvals, dedicated TOTP transport, uncertain authorization, session closure, expiry, peer-response binding, memory CRUD/sharing/isolation and disabled-by-default watch schedules.

## Chat polish

Live chat has a growing bottom composer, compact profile/model controls, independent transcript scrolling, Jump to latest, draft-only starter prompts, Copy feedback and inline interrupted-response recovery. Desktop Enter sends (Shift+Enter for newlines); touch Enter inserts a newline and Send submits. Stop does not guarantee server cancellation, so unconfirmed requests require history review before resending. Latest local verification: build, 14 unit/API checks and 67 desktop/phone browser checks pass, one intentional skip.

## Shared Claude API credential

App settings → Claude & models → Claude API now supports human-entered Anthropic tokens through a dedicated authenticated endpoint. Tokens are stored encrypted outside chat/settings history and never read back to the browser. New requests use the saved token immediately, ahead of environment/config fallback. The current LAN development URL uses HTTP; encryption at rest does not encrypt network transport. Production HTTPS acceptance remains separate. Use `scripts/dev-local.py --paired` to restart with private environment credentials while preserving paired access.
