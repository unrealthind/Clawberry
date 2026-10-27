#!/bin/sh
set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
tool_root="$project_root/.runtime/dev-tools"
compiler_root="$tool_root/env"

if [ ! -x "$tool_root/cargo/bin/cargo" ] || [ ! -x "$compiler_root/bin/node" ]; then
    echo 'Local development tools are missing; see docs/rocky-development.md.' >&2
    exit 1
fi

export CARGO_HOME="$tool_root/cargo"
export RUSTUP_HOME="$tool_root/rustup"
export PATH="$CARGO_HOME/bin:$compiler_root/bin:$PATH"
export CC="$compiler_root/bin/x86_64-conda-linux-gnu-cc"
export CXX="$compiler_root/bin/x86_64-conda-linux-gnu-c++"
export AR="$compiler_root/bin/x86_64-conda-linux-gnu-ar"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$CC"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export npm_config_cache="$tool_root/npm-cache"

exec "$@"
