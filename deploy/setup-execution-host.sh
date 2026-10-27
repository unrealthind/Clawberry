#!/bin/sh
# Run manually as host administrator. Never callable by an agent tool.
# Supports Debian/Ubuntu and Rocky/RHEL-family hosts.
set -eu

if [ "$(id -u)" -ne 0 ]; then
    echo 'Run: sudo sh deploy/setup-execution-host.sh' >&2
    exit 1
fi

install_packages() {
    if command -v dnf >/dev/null 2>&1; then
        # Rocky/RHEL 10 ships Podman 5. Rootless networking uses pasta/passt;
        # do not require the deprecated slirp4netns path.
        dnf install -y podman shadow-utils fuse-overlayfs passt ripgrep curl
    elif command -v apt-get >/dev/null 2>&1; then
        apt-get update
        apt-get install -y podman uidmap slirp4netns fuse-overlayfs ripgrep curl
    else
        echo 'Unsupported host: expected dnf or apt-get.' >&2
        exit 1
    fi
}

install_packages

install -d -o root -g root -m 0755 /etc/tinyclaw/security

# Do not replace a reviewed production policy on repeat runs.
if [ ! -e /etc/tinyclaw/security/root-policy.toml ]; then
    install -o root -g root -m 0644         config/root-policy.example.toml         /etc/tinyclaw/security/root-policy.toml
fi

# Refuse unsafe policy ownership/permissions. The runtime also verifies this,
# but deployment should fail early instead of discovering it after startup.
policy=/etc/tinyclaw/security/root-policy.toml
if [ -L "$policy" ]; then
    echo "Refusing symlinked root policy: $policy" >&2
    exit 1
fi
owner="$(stat -c '%u:%g' "$policy")"
mode="$(stat -c '%a' "$policy")"
if [ "$owner" != "0:0" ]; then
    echo "Root policy must be owned by root:root (found $owner)." >&2
    exit 1
fi
case "$mode" in
    *2|*3|*6|*7)
        echo "Root policy must not be group/world writable (mode $mode)." >&2
        exit 1
        ;;
esac

if ! command -v newuidmap >/dev/null 2>&1 || ! command -v newgidmap >/dev/null 2>&1; then
    echo 'Rootless Podman helpers newuidmap/newgidmap are missing after package installation.' >&2
    exit 1
fi

printf '%s\n' 'Installed rootless sandbox prerequisites and default-deny policy.'
printf '%s\n' 'Execution stays disabled until you review and install execution.toml and contracts.'
printf '%s\n' 'For rootless Podman, verify the runtime user has subordinate UID/GID ranges in /etc/subuid and /etc/subgid.'
