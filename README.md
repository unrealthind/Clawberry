# Clawberry

<p><img src="app/public/brand/logo.svg" alt="Clawberry" width="300" /></p>

**Run multiple persistent AI agents on your own hardware, with every external
permission under your control.**

Clawberry is a self-hosted, multi-agent assistant platform: a Rust runtime built
on [TinyClaw](https://github.com/suislanchez/tinyclaw) plus a React chat workspace.
Each agent has its own identity, conversations, memory, tasks and tool boundaries,
and can spin up temporary workers. Instead of handing a model your account keys,
external actions go through administrator-owned n8n workflows, and writes need
human approval plus a TOTP code.

Clawberry is the public continuation of RaaviClaw, published with a clean Git
history. Internal `raaviclaw` identifiers and the `tinyclaw` executable remain
for compatibility, and the screenshots below still show the RaaviClaw name.

> **Early-stage open source — contributors wanted.** The local foundations are
> implemented and heavily tested (thousands of Rust tests plus desktop/phone browser
> checks). Production deployment, live integrations and the remaining app work are
> unfinished. Run it in a development environment. See [Help finish it](#help-finish-it).

## Interface preview

The running app in **sample mode** (`?mode=sample`). Conversations, approvals and
usage figures are fictional; these do not demonstrate live integrations.

<picture>
  <source media="(prefers-color-scheme: light)" srcset="docs/screenshots/janet-chat-light.jpg">
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/janet-chat.jpg">
  <img src="docs/screenshots/janet-chat.jpg" alt="Janet's work conversation with the multi-agent sidebar, temporary workers, draft review, tasks and illustrative usage" width="1600">
</picture>

*Janet, the work agent. The sidebar shows both permanent agents and their running workers.*

<details>
<summary>More screens: Harley, Focus Threads, tasks, calendar and activity</summary>

### Harley — personal agent

![Harley's separate personal conversation and weekend planning draft](docs/screenshots/harley-chat.jpg)

### Focus Threads

![Agent-owned Focus Threads](docs/screenshots/focus-threads.jpg)

### Tasks

![Task cards with deadlines, checklists and progress](docs/screenshots/tasks.jpg)

### Calendar

![Sample monthly calendar with events and task deadlines](docs/screenshots/calendar.jpg)

### Activity

![Sample activity timeline](docs/screenshots/activity.jpg)

</details>

## What is here

- Separate work and personal assistants, Janet and Harley, with owner-scoped chats,
  tasks, Focus Threads, files and memory.
- Temporary workers, structured results, task archives and model usage budgets.
- A desktop-first responsive chat app with agent settings, activity, memory and
  approval screens.
- Runtime foundations for policy checks, Guard review, secret references, sandboxed
  execution, read-only watches and reviewed n8n contracts.
- Dedicated human approval flows for cross-agent exchanges and TOTP-authorized
  external writes. These require administrator configuration and live acceptance.

Janet and Harley are peers. Their private memory is isolated; exchanging information
requires human approval. Models must not receive raw secrets or bypass policy,
Guard, approvals or the n8n/TOTP write path.

## Security model: n8n as the integration boundary

Agents get the smallest practical permissions. They can research, plan and propose,
but external systems are reached only through specific n8n workflows that an
administrator defines, never through unrestricted API tokens or arbitrary URLs.

- **Reads** are limited to explicitly configured read-only workflows.
- **Writes** show the exact proposed action and scope, need human approval, then a
  TOTP code that unlocks only that operation or a bounded session.
- The gateway rejects undeclared operations, enforces allowlists and budgets,
  records an audit trail and fails closed when anything is uncertain.

```text
                    Web workspace (chat / tasks)
                               |
                    Authenticated Rust gateway
                               |
                      Agent orchestration
                     /         |         \
                Agent A     Agent B    Workers
                     \         |         /
               Private memory / scoped tools
                               |
                    Policy and Guard checks
                               |
                         Action broker
                     /                     \
           Read-only requests         Proposed writes
                   |                         |
           n8n read workflows         Human approval
                   |                         |
             Scoped results          TOTP verification
                                             |
                                 Bounded n8n write workflow
```

This is a security design, not a guarantee that a model can never cause harm. Safe
use depends on correct gateway configuration, isolation and review.

## Try it locally

Prerequisites: Linux, Rust **1.92** with rustfmt/Clippy, Node.js **22**, npm,
Python 3, a C/C++ toolchain, pkg-config and OpenSSL development headers. See
[Rocky development](docs/rocky-development.md) for native dependencies and the
repository-local toolchain wrapper. Other platforms need their own validation.

Clone the public repository after publication:

```sh
git clone https://github.com/unrealthind/Clawberry.git
cd Clawberry
```

From your clone's root directory:

```sh
cargo build --locked
npm --prefix app ci --ignore-scripts
npm --prefix app run build
```

To view the fictional sample workspace without provider credentials:

```sh
npm --prefix app run dev
# Open http://127.0.0.1:5173/?mode=sample
```

For actual local chat, copy `.env.example` to `.env.local` and set your own
Anthropic credential there. Keep that file private. Provider requests are billable.
Then start the loopback-only development app:

```sh
python3 scripts/dev-local.py --local
# Open http://127.0.0.1:8080
```

The launcher builds the backend/frontend and keeps private state under ignored
`.runtime/`. Stop it with Ctrl+C. Loopback development mode has no login and must
remain local. The launcher's default LAN mode changes a temporary firewall rule;
read [the app guide](app/README.md) before selecting it. Use HTTPS for remote access
and credential enrollment. Public internet deployment is not accepted yet.

The executable is still named `tinyclaw` for compatibility. Most app features need
the default full build; tiny/standard builds have reduced functionality.

## Help finish it

The project's author can't finish this alone, and help of any size is welcome.
Look for issues labelled
[`good first issue`](https://github.com/unrealthind/Clawberry/labels/good%20first%20issue)
and [`help wanted`](https://github.com/unrealthind/Clawberry/labels/help%20wanted),
then read [CONTRIBUTING.md](CONTRIBUTING.md), the
[contributor starting points](docs/contributor-starting-points.md) and the
[documentation index](docs/README.md).

Biggest open areas:

- **App completion:** files and images in the main chat (multimodal input), a
  review queue for execution/Guard requests, accessibility and phone layouts.
- **Live validation:** checking real Guard, sandbox and n8n behavior against the
  documented contracts.
- **Deployment:** HTTPS, service lifecycle, recovery and backup/restore.
- **Onboarding:** reproducible packaging, other Linux distros and macOS, clearer setup.

Discuss large changes in an issue before implementing them. Preserve ownership,
auditability and human approval boundaries. Historical phase plans and worklogs
describe earlier checkpoints; they are not a promise that every feature is enabled.

## Verify changes

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
npm --prefix app run build
npm --prefix app test
```

Browser checks need Chromium and its native dependencies. From `app/`, run
`npx playwright install chromium --only-shell` and `npm run test:e2e`.
Rootless sandbox and live integration checks have separate acceptance procedures;
ordinary unit and browser fixtures do not certify them.

Report vulnerabilities privately using [SECURITY.md](SECURITY.md). Never submit
credentials, personal records, private screenshots, runtime databases or logs.

## License and origins

Clawberry is distributed under **GPL-3.0-only**; see [LICENSE](LICENSE).
It builds on [TinyClaw](https://github.com/suislanchez/tinyclaw), itself derived from
[ZeroClaw](https://github.com/theonlyhennygod/zeroclaw). Inherited MIT notices are
preserved in [LICENSE-MIT](LICENSE-MIT) and explained in [NOTICE](NOTICE).
Third-party dependencies and separately marked assets retain their own licenses.
