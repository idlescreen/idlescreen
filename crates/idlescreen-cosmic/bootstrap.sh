#!/usr/bin/env bash
set -euo pipefail

if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update && sudo apt-get install -y libdbus-1-dev libwayland-dev libxkbcommon-dev libssl-dev pkg-config libudev-dev
fi

if [ ! -d "../runtime" ] && [ "$(basename "$PWD")" != "runtime" ]; then git clone https://github.com/idlescreen/runtime ../runtime; fi

# Cargo.toml declares four path deps into runtime (idle-dbus, idle-err, idle-log,
# idle-runner), all under runtime/. CI checks the engine out there; locally it
# is a gitignored symlink to the sibling clone. Without this the build fails
# on a fresh machine with a confusing "unable to update <repo>/runtime/..."
# for every one of the four path deps.
if [ ! -e runtime ] && [ -d ../runtime ]; then ln -s ../runtime runtime; fi

if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck source=/dev/null
        source "$HOME/.cargo/env"
    fi
fi

rustup show >/dev/null

cargo test
