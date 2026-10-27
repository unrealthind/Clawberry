# Clawberry assistant-ui workspace

October 20 status: this is the current application, not a separate legacy shell. [Phases 0–4](../../../docs/eight-phase-plan.md) are complete locally; Phase 5 extends attachments, evidence panels, shared-memory controls and conversational cards. Backend availability does not imply those app flows are complete. Current build/14 unit checks/67 browser checks pass (one intentional skip); production activation remains deferred.

Run `npm ci --ignore-scripts`, then `npm run dev:prototype` from `app/`. Open `/` for live data; `/prototype.html` loads the same application. Start the backend with `python3 scripts/dev-local.py --build` from the repository root. Use `/prototype.html?mode=sample` for explicit fictional samples. The legacy management shell has been retired.

The workspace integrates MIT-licensed `@assistant-ui/react` with React/Vite, Lucide icons, and Clawberry styling. assistant-ui handles sample conversation state, composer, scripted streaming and cancellation. Janet/Harley main chats and Focus Threads have separate local runtimes. All rich workspace components are predefined React UI; agents do not generate executable UI.

See [the current scope](../../../docs/ui-workspace.md) for data lifetimes, live versus sample features, authorization, remaining integration and verification.

## Settings and setup

Global app settings open from the explicitly labeled App settings footer or Command/Control + comma: appearance, optional notifications, calendar preferences, connection/security, and shared Claude API/model profiles. There is no duplicate Settings entry in workspace navigation.

The settings control beside Janet or Harley opens that owner’s consolidated setup page: Identity & memory, Skills, Credentials, and Access & boundaries. Personality and guiding principles use the existing soul.md editor; selected memory notes use memory.md. There is no duplicate personality form, or separate workspace Skills/Vault entry. Shared API/model profiles stay global by User’s explicit preference. Project/conversation Files remains separate.

Model selectors include the repository’s existing defaults plus models listed in the [official Claude model reference](https://platform.claude.com/docs/en/models/overview), checked October 18, 2026. Availability for a specific account is unverified. Live profile and budget saves use versioned backend settings; explicit sample mode retains browser-local drafts. API-key enrollment remains disabled until secure backend integration; the browser makes no direct provider requests.

## Sample workspace

Task creation/editing, priorities, completion/reopening, optional deadlines, filters and calendar views share top-level state. Each thread has conversation and workspace views collecting linked tasks, filenames, human-recorded decisions and fictional worker handoffs. Sample runs and approvals do not execute external actions. Main-chat summaries remain compact. Calendar event times are labeled America/Edmonton and sample today is October 16, 2026.

Conversation, task, calendar, thread and worker state resets on reload. Explicitly saved skills/core files and nonsecret model drafts persist in local storage. These namespaces support UI exploration, not proof of runtime isolation. Unsaved file edits survive in-app navigation; browser reload discards them. Invalid saved data is preserved and blocked from overwrite; save failures leave draft edits visible.

Skills are plain-text library/file editing and download previews, not installed or executed. Limits remain 16 skills, 20 files per skill, 64 KB per file and 512 KB serialized per agent. Core files are capped at 32,768 characters each and cannot be removed through this UI. Credential references and injection previews contain fictional metadata only; no credential secret is accepted, revealed, injected or transmitted. Pausing sample access does not change runtime permissions.

Light/dark appearance is encoded in the URL (`?theme=light` / `?theme=dark`). Context defaults open on wide screens and closed at compact sizes; setup/settings and calendar use the full workspace width. Chat activity follows actual local generation/cancellation, running sample workers, then queued tasks. Sample Claude usage/cost cards are illustrative, not billing or current provider pricing. Portraits and prompts are documented in [asset notes](../../public/prototype/agents/README.md).

## Opt-in gateway connection

Live data is the normal workspace. Global Settings → Connection & security → Connect gateway also opens it from explicit sample mode. On localhost, there is no token entry: start the loopback gateway with `--local-ui`. Local requests omit bearer headers. On remote hosts, enter a paired token only in its dedicated password form; it stays in memory and clears on disconnect/reload. Same-origin API routes use the existing configured gateway; there is no caller-supplied endpoint. Agent switching/unmount aborts chat/history reads; dispatched server operations may still finish.

The connected view supports SSE chat, cursor-paginated history, actual Fast/Normal/Deep profile requests, Focus Thread creation/navigation, task create/edit/complete, calendar deadline projection and read-only activity. Thread context groups the persisted summary, linked tasks, file metadata and matching worker statuses. Activity/file failures do not block chat/tasks. Desktop Enter sends and Shift+Enter inserts a newline; touch Enter inserts a newline and Send submits. It requires the `task.edit` backend operation in this change. Live terminal tasks remain immutable. Thread file transfers, durable decisions, worker archive inspection and server-backed global/agent settings are connected. External calendar sync and credential enrollment remain unfinished. Approvals use the current workspace with the dedicated TOTP/session controls.

No automatic task mutation retry is performed. If creation succeeds but details fail, its known ID is reused on save. Unconfirmed creation blocks blind resubmission until the dialog closes and tasks are inspected. Unconfirmed chat asks for a history refresh before resending.

## Checks

```bash
npm run build
npm test
npx prettier --check src e2e playwright.config.ts vite.config.ts tsconfig.json package.json index.html
npm run test:e2e
```

The production build includes both entry points. Unit/API coverage includes closed nonsecret model-draft validation, storage failures, and invalid saved-data preservation. Browser coverage includes desktop/phone conversation ownership, task lifecycle and deadlines, thread evidence, file/skill editing, credential previews, model draft persistence, settings separation, recovery and overflow. Current final-head evidence belongs in [merge readiness](../../../docs/merge-readiness.md), not a claim of live deployment acceptance. Existing third-party Zod annotation and prototype bundle-size warnings remain.

## Durable settings (October 19)

Live App settings footer settings persist global preferences and actual Claude model profiles/budgets through the backend. Agent gear settings persist owner-only soul.md, memory.md notes, default profile and skill text packages. Saves are explicit, revision checked, and applied to subsequent runtime requests; conflicts retain the draft. Sample local-storage drafts are not imported automatically. Development credentials come from the local environment/.env.local; production web enrollment remains planned, and Vault is disabled. See [product plan](../../../docs/product-plan.md).

## October 19 thread and recovery delivery

Step 4 is complete locally: authenticated thread file upload/download, schema-v9 durable human decisions with idempotent retries, and structured worker results/archive downloads. Local validation: 2,445 Rust tests and 14 frontend checks passed; 43 browser checks passed with one intentional skip; five existing host sandbox Rust tests remain ignored. Both gateway modes passed complete stopped fixture-state backup/restore. The Linux service unit newline bug is fixed, and concurrent write-budget admission is tested across independent connections.

Step 5 is deferred by User; target VM, live Guard/n8n/TOTP and always-on acceptance remain open. Production credential enrollment remains unfinished. Final-head CI is required; At that historical checkpoint PR #1 was draft and main was unchanged; the later authorized consolidation superseded that restriction. See [deployment acceptance](../../../docs/deployment-acceptance.md).

## Connected UI consistency

Live pages share the preview’s control/card styling and section navigation. The App settings footer visibly labels the global scope and shows a gear/selected state; owner gears open separate agent setup. General restores appearance cards, Manage models and keyboard shortcuts. Connection & security shows safe connection/provider metadata. Agent Credentials is present as a read-only, unconnected section and collects no secrets.

The header theme switch persists with backend revision checks; settings conflicts retain newer server data and surface an error. Opening a view resets scroll so the new page is visible. On phones, the navigation drawer contains keyboard focus while open and is inert while closed. Build, 14 unit checks and 45 browser checks pass (one intentional browser skip).

## Unified management screens

Approvals and Memory now use the current shell at both `/` and `/prototype.html`. There are no links to a separate management UI. The legacy shell/CSS are removed. Watch settings and recommendations are integrated into Activity. Native dialogs and existing design tokens keep memory controls consistent; TOTP still uses the dedicated authorization helper and never a normal operation/cache. Disconnect returns to Connect, while sample mode requires explicit selection. Build/14 unit checks and 57 browser checks pass (one intentional browser skip). Deployment acceptance remains deferred.

## Live chat polish

A growing bottom composer keeps profiles and model metadata compact. The transcript scrolls independently, follows new replies only while reading the latest content, and offers Jump to latest. Empty-state prompts fill drafts; reply copying reports success/failure. Interrupted responses retain the draft and partial text with a required history refresh before resending. Stop cancels browser reception, not guaranteed server work. Desktop completion restores input focus; touch input keeps Enter for newlines. Build, 14 unit/API tests and 67 browser checks pass (one intentional skip). Chat attachments and rich Markdown remain planned.
