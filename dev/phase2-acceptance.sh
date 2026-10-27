#!/bin/sh
set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

: "${RAAVICLAW_SANDBOX_IMAGE:?Set RAAVICLAW_SANDBOX_IMAGE to a reviewed, locally installed image digest}"
command -v podman >/dev/null 2>&1 || {
    echo 'Install the rootless Podman prerequisites through your host terminal first.' >&2
    exit 1
}
[ "$(podman info --format '{{.Host.Security.Rootless}}')" = true ] || {
    echo 'Acceptance requires rootless Podman; host-shell fallback is forbidden.' >&2
    exit 1
}
podman image exists "$RAAVICLAW_SANDBOX_IMAGE"
RAAVICLAW_PACKAGE_TEST_IMAGE=$(python3 dev/build-package-fixture.py "$RAAVICLAW_SANDBOX_IMAGE")
export RAAVICLAW_PACKAGE_TEST_IMAGE

sh dev/rocky-run.sh cargo test --locked --test sandbox_acceptance -- \
    --ignored --skip real_host_policy_loads_and_remains_default_deny --test-threads=1
sh dev/rocky-run.sh cargo test --locked --lib \
    security::execution::tests::real_sandbox_dispatch_requires_single_use_broker_admission -- \
    --ignored --test-threads=1

if [ -f /etc/tinyclaw/security/root-policy.toml ]; then
    sh dev/rocky-run.sh cargo test --locked --test sandbox_acceptance \
        real_host_policy_loads_and_remains_default_deny -- --ignored
else
    echo 'Host-owned policy acceptance remains pending: no policy is installed.' >&2
    exit 1
fi
