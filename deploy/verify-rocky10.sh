#!/bin/sh
# Non-destructive Rocky Linux 10 deployment acceptance checks.
# Run as the same unprivileged account that runs RaaviClaw.
set -eu

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}
pass() {
    printf 'PASS: %s\n' "$*"
}

[ "$(id -u)" -ne 0 ] || fail "run this script as the unprivileged RaaviClaw service account"

# Standard system-provided OS metadata.
# shellcheck disable=SC1091
. /etc/os-release
[ "${ID:-}" = "rocky" ] || fail "expected Rocky Linux, found ${ID:-unknown}"
case "${VERSION_ID:-}" in
    10|10.*) ;;
    *) fail "expected Rocky Linux 10, found ${VERSION_ID:-unknown}" ;;
esac
pass "Rocky Linux 10"

command -v getenforce >/dev/null 2>&1 || fail "getenforce is unavailable"
[ "$(getenforce)" = "Enforcing" ] || fail "SELinux must remain Enforcing"
pass "SELinux enforcing"

command -v podman >/dev/null 2>&1 || fail "podman is not installed"
podman info >/dev/null 2>&1 || fail "rootless podman is not usable by $(id -un)"
if podman info --format '{{.Host.Security.Rootless}}' 2>/dev/null | grep -qx true; then
    pass "rootless Podman"
else
    fail "Podman is not running rootless"
fi

user="$(id -un)"
grep -q "^${user}:" /etc/subuid || fail "missing /etc/subuid range for $user"
grep -q "^${user}:" /etc/subgid || fail "missing /etc/subgid range for $user"
command -v newuidmap >/dev/null 2>&1 || fail "newuidmap is unavailable"
command -v newgidmap >/dev/null 2>&1 || fail "newgidmap is unavailable"
pass "subordinate UID/GID mappings"

command -v loginctl >/dev/null 2>&1 || fail "loginctl is unavailable"
if [ "$(loginctl show-user "$user" -p Linger --value 2>/dev/null)" = "yes" ]; then
    pass "systemd user lingering enabled"
else
    fail "systemd user lingering is disabled; run as root: loginctl enable-linger $user"
fi

policy=/etc/tinyclaw/security/root-policy.toml
[ -f "$policy" ] || fail "root policy is missing"
[ ! -L "$policy" ] || fail "root policy must not be a symlink"
[ "$(stat -c '%u:%g' "$policy")" = "0:0" ] || fail "root policy must be root:root"
mode="$(stat -c '%a' "$policy")"
case "$mode" in
    *2|*3|*6|*7) fail "root policy is group/world writable (mode $mode)" ;;
esac
pass "root policy ownership and permissions"

command -v tinyclaw >/dev/null 2>&1 || fail "tinyclaw binary is not installed in PATH"
tinyclaw --version >/dev/null 2>&1 || fail "tinyclaw binary failed to execute"
pass "RaaviClaw binary"

if systemctl --user is-active --quiet tinyclaw.service; then
    pass "systemd user service active"
else
    fail "tinyclaw.service is not active for $user"
fi

command -v curl >/dev/null 2>&1 || fail "curl is required for gateway acceptance"
health="$(curl --fail --silent --show-error --max-time 5 http://127.0.0.1:8080/health)" ||
    fail "local gateway health endpoint failed"
printf '%s' "$health" | grep -q '"status":"ok"' || fail "gateway health did not report ok"
pass "local gateway health"

printf '\nRocky Linux 10 deployment acceptance: PASS\n'
