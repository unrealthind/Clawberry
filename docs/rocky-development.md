# Rocky VM development

This Rocky Linux VM on Proxmox is User's development machine. Development validation does not install an always-on service or resume deferred production deployment acceptance.

October 20: local Phases 0–4 are complete; [the current plan](eight-phase-plan.md) and [documentation index](README.md) distinguish app work from deferred production gates. Final Phase 4 evidence is under `.runtime/phase4-evidence/`: full/tiny/standard 2,494/1,768/1,768, strict Clippy/formatting, debug/release recovery, 14 frontend and 67 browser checks (one intentional skip). Invoke the wrapper with `sh`; browser runs require the repository-local `PLAYWRIGHT_BROWSERS_PATH` shown below. Avoid concurrent feature builds and daemon smoke tests sharing `target/debug/tinyclaw`: tiny/standard builds can replace the full-feature binary. Build the full profile immediately before daemon smoke.

## Tools

The repository pins Rust 1.92. CI uses Node 22. On a new Rocky VM, User can install native build and headless-browser prerequisites in a terminal:

```sh
sudo dnf install -y gcc gcc-c++ make nodejs npm openssl-devel pkgconf-pkg-config nss nspr atk at-spi2-atk cups-libs libdrm libXcomposite libXdamage libXfixes libXrandr mesa-libgbm libxkbcommon alsa-lib pango cairo
```

The October 19 development checks use isolated tools below the git-ignored `.runtime/dev-tools` directory. They require no sudo, change no shell profiles, and do not contain provider credentials. To recreate them from the repository root:

```sh
mkdir -p .runtime/dev-tools
curl -fsSL https://micro.mamba.pm/api/micromamba/linux-64/latest -o /tmp/raaviclaw-micromamba.tar.bz2
tar -xjf /tmp/raaviclaw-micromamba.tar.bz2 -C .runtime/dev-tools bin/micromamba
.runtime/dev-tools/bin/micromamba create -y -r "$PWD/.runtime/dev-tools/mamba" -p "$PWD/.runtime/dev-tools/env" -c conda-forge gcc_linux-64 gxx_linux-64 nodejs=22 pkg-config
curl -fsSL https://sh.rustup.rs -o /tmp/raaviclaw-rustup-init.sh
CARGO_HOME="$PWD/.runtime/dev-tools/cargo" RUSTUP_HOME="$PWD/.runtime/dev-tools/rustup" sh /tmp/raaviclaw-rustup-init.sh -y --no-modify-path --profile minimal --default-toolchain 1.92 --component rustfmt,clippy
```

`sh dev/rocky-run.sh COMMAND ...` selects those tools, the C linker and a default of two Cargo jobs for this VM's memory limit. It does not load `.env.local`, start services or grant agent execution permissions. Package downloads require network access; subsequent cached Cargo checks can use `--offline`.

These are local development builds: the isolated compiler links its support library from `.runtime/dev-tools/env/lib`. Keep that directory while using these binaries. Production packaging must build and validate against its own target toolchain rather than copying this development binary.

## Baseline checks

From the repository root:

```sh
sh dev/rocky-run.sh cargo fmt --all -- --check
sh dev/rocky-run.sh cargo clippy --locked --all-targets -- -D warnings
sh dev/rocky-run.sh cargo test --locked --all-targets
sh dev/rocky-run.sh cargo test --locked --no-default-features --features tiny --all-targets
sh dev/rocky-run.sh cargo test --locked --no-default-features --features standard --all-targets
sh dev/rocky-run.sh cargo build --locked
sh dev/rocky-run.sh cargo build --release --locked
python3 scripts/test_assistant_smoke.py
python3 scripts/test_assistant_smoke.py --local-ui
python3 scripts/test_tui_smoke.py
```

The smoke scripts create temporary state and use a local fixture provider or offline PTY. They do not need real credentials or call billable models. The assistant smoke checks authentication, agent ownership, persistence and restart recovery. Rootless-container and host-policy tests are opt-in: run `dev/phase2-acceptance.sh` with the reviewed image after manual host setup. These checks passed locally on October 20; see `eight-phase-plan.md` for evidence and remaining live/production gates.

From `app/`:

```sh
sh ../dev/rocky-run.sh npm ci --ignore-scripts
sh ../dev/rocky-run.sh npm run build
sh ../dev/rocky-run.sh npm test
sh ../dev/rocky-run.sh npx prettier --check src e2e playwright.config.ts vite.config.ts tsconfig.json package.json index.html
PLAYWRIGHT_BROWSERS_PATH="$PWD/../.runtime/dev-tools/browsers" sh ../dev/rocky-run.sh npx playwright install chromium --only-shell
PLAYWRIGHT_BROWSERS_PATH="$PWD/../.runtime/dev-tools/browsers" sh ../dev/rocky-run.sh npm run test:e2e
```

Browser tests require the native libraries listed above. Browser fixtures do not certify live Guard, n8n/TOTP or production deployment.

## Local startup

After building the backend and frontend, use a disposable fixture config for offline validation. For real local chat, configure an approved provider credential through the existing environment or private `.env.local`, then run:

```sh
sh dev/rocky-run.sh python3 scripts/dev-local.py
```

This starts the existing direct-loopback development gateway at `http://127.0.0.1:8080`. Stop it with Ctrl+C; rerun the command to restart using the existing `.runtime` state. It is deliberately not a system service. Keep the local-only access boundary; remote access needs the separately reviewed authentication/deployment path. Do not overwrite existing runtime data with test fixtures.
