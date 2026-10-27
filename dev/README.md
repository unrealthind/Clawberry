# RaaviClaw Development Environment

For the current Rocky development VM, use [Rocky development setup and baseline checks](../docs/rocky-development.md). The container instructions below are inherited alternatives.

Real local security/sandbox acceptance is reproducible with `dev/phase2-acceptance.sh`; scope and completed evidence are in [the eight-phase plan](../docs/eight-phase-plan.md).

For Phase 3/4 protocol, memory, watch and peer checks see [connected operations](../docs/connected-operations.md) and [source-aware foundation](../docs/source-aware-foundation.md). Use `sh dev/rocky-run.sh` for pinned tools and the repository-local Chromium path documented in the Rocky guide. Build/test feature profiles sequentially before daemon smoke so a tiny binary cannot replace the full-feature daemon during a check. These fixture checks use temporary state and grant no production capability.

Current delivery status — October 20, 2026: [eight-phase plan](../docs/eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

A fully containerized development sandbox for ZeroClaw agents. This environment allows you to develop, test, and debug the agent in isolation without modifying your host system.

## Directory Structure

- **`agent/`**: (Merged into root Dockerfile)
  - The development image is built from the root `Dockerfile` using the `dev` stage (`target: dev`).
  - Based on `debian:bookworm-slim` (unlike production `distroless`).
  - Includes `bash`, `curl`, and debug tools.
- **`sandbox/`**: Dockerfile for the simulated user environment.
  - Based on `ubuntu:22.04`.
  - Pre-loaded with `git`, `python3`, `nodejs`, `npm`, `gcc`, `make`.
  - Simulates a real developer machine.
- **`docker-compose.yml`**: Defines the services and `dev-net` network.
- **`cli.sh`**: Helper script to manage the lifecycle.

## Usage

Run all commands from the repository root using the helper script:

### 1. Start Environment
```bash
./dev/cli.sh up
```
Builds the agent from source and starts both containers.

### 2. Enter Agent Container (`zeroclaw-dev`)
```bash
./dev/cli.sh agent
```
Use this to run `zeroclaw` CLI commands manually, debug the binary, or check logs internally.
- **Path**: `/zeroclaw-data`
- **User**: `nobody` (65534)

### 3. Enter Sandbox (`sandbox`)
```bash
./dev/cli.sh shell
```
Use this to act as the "user" or "environment" the agent interacts with.
- **Path**: `/home/developer/workspace`
- **User**: `developer` (sudo-enabled)

### 4. Development Cycle
1. Make changes to Rust code in `src/`.
2. Rebuild the agent:
   ```bash
   ./dev/cli.sh build
   ```
3. Test changes inside the container:
   ```bash
   ./dev/cli.sh agent
   # inside container:
   zeroclaw --version
   ```

### 5. Persistence & Shared Workspace
The local `playground/` directory (in repo root) is mounted as the shared workspace:
- **Agent**: `/zeroclaw-data/workspace`
- **Sandbox**: `/home/developer/workspace`

Files created by the agent are visible to the sandbox user, and vice versa.

The agent configuration lives in `target/.zeroclaw` (mounted to `/zeroclaw-data/.zeroclaw`), so settings persist across container rebuilds.

### 6. Cleanup
Stop containers and remove volumes and generated config:
```bash
./dev/cli.sh clean
```
**Note:** This removes `target/.zeroclaw` (config/DB) but leaves the `playground/` directory intact. To fully wipe everything, manually delete `playground/`.
