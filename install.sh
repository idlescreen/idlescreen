#!/bin/sh
# IdleScreen Installer — Standalone installation script for the native .oo suite.
# Usage:
#   ./install.sh                       install / update from a source checkout
#   IDLESCREEN_INSTALL_BINARY=/path/to/idlescreen ./install.sh
#                                      install a specific build
#   ./install.sh --uninstall           remove IdleScreen, keeping user configuration
#   ./install.sh --purge               with --uninstall: also remove user config
set -eu

DRY_RUN=0
UNINSTALL=0
UNINSTALL_PURGE=0

for arg in "$@"; do
    case "$arg" in
        --plan|--dry-run) DRY_RUN=1 ;;
        --uninstall) UNINSTALL=1 ;;
        --purge) UNINSTALL_PURGE=1 ;;
        -h|--help)
            cat <<'USAGE'
Usage: install.sh [options]

  (no options)        install / update IdleScreen (all 53 scenes built-in)
  --uninstall         remove IdleScreen, keeping user configuration
  --purge             with --uninstall: also remove user and system config
  --plan | --dry-run  print the install plan and exit
  --verify            report system requirements and the binary digest

Environment:
  IDLESCREEN_INSTALL_BINARY   path to the idlescreen build to install
  XDG_CONFIG_HOME             overrides ~/.config for the config and unit paths
USAGE
            exit 0
            ;;
        --verify)
            printf 'IdleScreen installer verification:\n'
            printf '  - Host: %s (%s)\n' "$(uname -s 2>/dev/null || echo Linux)" "$(uname -m 2>/dev/null || echo x86_64)"
            if [ -d /run/systemd/system ]; then
                printf '  - Service manager: systemd user session supported\n'
            else
                printf '  - Service manager: no systemd user manager (dinit or XDG autostart will be used)\n'
            fi
            if command -v sha256sum >/dev/null 2>&1; then
                printf '  - Binary digest: %s\n' "$(sha256sum "${IDLESCREEN_INSTALL_BINARY:-./bin/idlescreen}" 2>/dev/null || echo unavailable)"
            else
                printf '  - Binary digest: unavailable (no sha256sum)\n'
            fi
            printf '  - Note: no signature is verified here; verify the digest against the release notes.\n'
            exit 0
            ;;
    esac
done

# Terminal colors
if [ -t 1 ]; then
    BOLD="\033[1m"; DIM="\033[2m"; RESET="\033[0m"
    GREEN="\033[32m"; CYAN="\033[36m"; YELLOW="\033[33m"; RED="\033[31m"
else
    BOLD=""; DIM=""; RESET=""; GREEN=""; CYAN=""; YELLOW=""; RED=""
fi

say() { printf '%b\n' "$*"; }
step() { say "  ${CYAN}${BOLD}$*${RESET}"; }
ok() { say "  ${GREEN}✓${RESET} $*"; }
warn() { say "  ${YELLOW}!${RESET} $*"; }
err() { say "  ${RED}✗${RESET} $*" >&2; }

banner() {
    say "${CYAN}${BOLD}"
    say "        ██╗██████╗ ██╗     ███████╗███████╗ ██████╗██████╗ ███████╗███████╗███╗   ██╗"
    say "        ██║██╔══██╗██║     ██╔════╝██╔════╝██╔════╝██╔══██╗██╔════╝██╔════╝████╗  ██║"
    say "        ██║██║  ██║██║     █████╗  ███████╗██║     ██████╔╝█████╗  █████╗  ██╔██╗ ██║"
    say "        ██║██║  ██║██║     ██╔══╝  ╚════██║██║     ██╔══██╗██╔══╝  ██╔══╝  ██║╚██╗██║"
    say "        ██║██████╔╝███████╗███████╗███████║╚██████╗██║  ██║███████╗███████╗██║ ╚████║"
    say "        ╚═╝╚═════╝ ╚══════╝╚══════╝╚══════╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝╚═╝  ╚═══╝"
    say "                          16-Bit Arcade Screensaver Suite (v5.0.0)"
    say "${RESET}"
}

# XDG-aware config root: XDG_CONFIG_HOME, then ~/.config.
config_root() {
    if [ -n "${XDG_CONFIG_HOME:-}" ]; then
        printf '%s' "${XDG_CONFIG_HOME}"
    else
        printf '%s' "${HOME}/.config"
    fi
}

# Remove the alias symlinks that older releases shipped. The zero-symlink law
# means no release may leave one behind.
remove_legacy_aliases() {
    for _alias in idle-daemon idle-cli idle-tui idlescreen-tui idlescreen-daemon; do
        rm -f "/usr/local/bin/${_alias}" "/usr/bin/${_alias}" "${HOME}/.local/bin/${_alias}" 2>/dev/null || true
    done
}

uninstall_action() {
    step "Removing IdleScreen..."
    systemctl --user disable --now idlescreen.service 2>/dev/null || true
    dinitctl disable idlescreen 2>/dev/null || true
    rm -f "$(config_root)/systemd/user/idlescreen.service"
    rm -f "$(config_root)/autostart/idlescreen.desktop"
    systemctl --user daemon-reload 2>/dev/null || true
    remove_legacy_aliases
    if [ -w /usr/local/bin ]; then
        rm -f /usr/local/bin/idlescreen
    elif command -v sudo >/dev/null 2>&1; then
        sudo rm -f /usr/local/bin/idlescreen 2>/dev/null || true
    fi
    rm -f "${HOME}/.local/bin/idlescreen"
    rmdir "$(config_root)/systemd/user" 2>/dev/null || true
    rmdir "$(config_root)/autostart" 2>/dev/null || true
    if [ "$UNINSTALL_PURGE" -eq 1 ]; then
        rm -rf "$(config_root)/idlescreen"
        ok "Purged configuration ($(config_root)/idlescreen)"
    fi
    ok "IdleScreen uninstalled successfully."
    exit 0
}

if [ "$UNINSTALL" -eq 1 ]; then
    uninstall_action
fi

banner
step "[1/5]  Scanning host identity"
OS_NAME=$(grep '^PRETTY_NAME=' /etc/os-release 2>/dev/null | cut -d= -f2 | tr -d '"' || uname -s)
ARCH=$(uname -m)
say "  ${DIM}host${RESET}     ${GREEN}${OS_NAME}${RESET} ${DIM}(${ARCH})${RESET}"

TARGET_BIN="${HOME}/.local/bin/idlescreen"
if [ -w /usr/local/bin ]; then
    TARGET_BIN="/usr/local/bin/idlescreen"
elif command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
    TARGET_BIN="/usr/local/bin/idlescreen"
fi

CFG_ROOT="$(config_root)"
CFG_DIR="${CFG_ROOT}/idlescreen"

if [ "$DRY_RUN" -eq 1 ]; then
    say "  ${DIM}plan${RESET}     Install idlescreen to ${TARGET_BIN}"
    say "  ${DIM}plan${RESET}     Configuration directory: ${CFG_DIR}"
    if [ -d /run/systemd/system ]; then
        say "  ${DIM}plan${RESET}     Configure systemd user service"
    else
        say "  ${DIM}plan${RESET}     Configure dinit or XDG autostart service"
    fi
    say "Dry run complete. Exiting."
    exit 0
fi

step "[2/5]  Locating the IdleScreen binary"
SCRIPT_DIR="$(cd "$(dirname "$0")" 2>/dev/null && pwd || echo .)"
BIN_PATH=""

# A source checkout is the only place a build is taken from. The current
# directory is never trusted on its own: a piped install runs from wherever the
# user happens to be, and an unrelated ./bin/idlescreen there is not ours.
IS_CHECKOUT=0
if [ -f "${SCRIPT_DIR}/main.oo" ] && [ -f "${SCRIPT_DIR}/Makefile" ]; then
    IS_CHECKOUT=1
fi

if [ -n "${IDLESCREEN_INSTALL_BINARY:-}" ]; then
    if [ ! -x "${IDLESCREEN_INSTALL_BINARY}" ]; then
        err "IDLESCREEN_INSTALL_BINARY is not an executable file: ${IDLESCREEN_INSTALL_BINARY}"
        exit 1
    fi
    BIN_PATH="${IDLESCREEN_INSTALL_BINARY}"
elif [ "$IS_CHECKOUT" -eq 1 ] && [ -x "${SCRIPT_DIR}/bin/idlescreen" ]; then
    BIN_PATH="${SCRIPT_DIR}/bin/idlescreen"
elif [ "$IS_CHECKOUT" -eq 1 ] && command -v oodac >/dev/null 2>&1; then
    step "Building idlescreen from native source..."
    (cd "$SCRIPT_DIR" && make build OODA_COMPILER="$(command -v oodac)")
    BIN_PATH="${SCRIPT_DIR}/bin/idlescreen"
fi

if [ -z "$BIN_PATH" ]; then
    err "No installable IdleScreen build found."
    if [ "$IS_CHECKOUT" -eq 1 ]; then
        err "The source build failed. Check the output above."
    else
        err "This installer will not copy a binary found on \$PATH: it may be an older release."
        err "Install from a checkout: git clone https://github.com/idlescreen/idlescreen && cd idlescreen && ./install.sh"
        err "Or point IDLESCREEN_INSTALL_BINARY at the build you want to install."
    fi
    exit 1
fi

if [ "$TARGET_BIN" = "/usr/local/bin/idlescreen" ]; then
    if [ -w /usr/local/bin ]; then
        cp -f "$BIN_PATH" "$TARGET_BIN"
        chmod 755 "$TARGET_BIN"
    else
        sudo cp -f "$BIN_PATH" "$TARGET_BIN"
        sudo chmod 755 "$TARGET_BIN"
    fi
else
    mkdir -p "${HOME}/.local/bin"
    cp -f "$BIN_PATH" "$TARGET_BIN"
    chmod 755 "$TARGET_BIN"
fi
ok "Installed binary to ${BOLD}${TARGET_BIN}${RESET}"

step "[3/5]  Configuring user settings"
mkdir -p "$CFG_DIR"
if [ ! -f "${CFG_DIR}/config.toml" ]; then
    cat <<'EOF' > "${CFG_DIR}/config.toml"
[general]
idle_enabled = true
idle_timeout_secs = 120
dpms_timeout_secs = 0
primary_output = ""

[saver]
active = "ascii"
custom_text = ""
use_os_brand = true

[saver.ascii]
mode = "os"
targets = ["os"]
text = ""
fps = 60
dwell_secs = 5
dissolve_duration_secs = 0.7
EOF
    ok "Created default configuration: ${CFG_DIR}/config.toml"
else
    ok "Preserved existing configuration: ${CFG_DIR}/config.toml"
fi

step "[4/5]  Installing user service"
if [ -d /run/systemd/system ]; then
    UNIT_DIR="${CFG_ROOT}/systemd/user"
    mkdir -p "$UNIT_DIR"
    cat <<EOF > "${UNIT_DIR}/idlescreen.service"
[Unit]
Description=IdleScreen screensaver daemon
Documentation=https://github.com/idlescreen/idlescreen
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
Slice=background-graphical.slice
SyslogIdentifier=idlescreen
StandardOutput=journal
StandardError=journal
ExecStartPre=-/usr/bin/mkdir -p ${CFG_DIR}
ExecStart=${TARGET_BIN} daemon
Restart=on-failure
RestartSec=5
TimeoutStopSec=10
KillMode=mixed

NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=%t -%h/.config -%h/.config/idlescreen -%h/.config/idle -%h/.local/state -%h/.local/state/idlescreen -%h/.cache -%h/.cache/idlescreen
PrivateTmp=true
ProtectHome=read-only
MemoryDenyWriteExecute=true
RestrictAddressFamilies=AF_UNIX AF_INET
IPAddressAllow=localhost 127.0.0.1
IPAddressDeny=any
MemoryMax=512M
TasksMax=128
ProtectControlGroups=strict
SystemCallFilter=@system-service
RestrictNamespaces=true
LockPersonality=true
RestrictRealtime=true
RestrictSUIDSGID=true
SystemCallArchitectures=native

[Install]
WantedBy=graphical-session.target
EOF
    systemctl --user daemon-reload 2>/dev/null || true
    systemctl --user enable --now idlescreen.service 2>/dev/null || true
    ok "Installed systemd user unit: ${UNIT_DIR}/idlescreen.service"
    SERVICE_KIND="systemd user scope"
else
    AUTOSTART_DIR="${CFG_ROOT}/autostart"
    mkdir -p "$AUTOSTART_DIR"
    cat <<EOF > "${AUTOSTART_DIR}/idlescreen.desktop"
[Desktop Entry]
Type=Application
Name=IdleScreen
Comment=IdleScreen screensaver daemon
Exec=${TARGET_BIN} daemon
Terminal=false
X-GNOME-Autostart-enabled=true
EOF
    dinitctl enable idlescreen 2>/dev/null || true
    ok "Installed XDG autostart entry: ${AUTOSTART_DIR}/idlescreen.desktop"
    SERVICE_KIND="XDG autostart"
fi

step "[5/5]  Starting the idle user service"
_ready=0
_i=0
while [ "$_i" -lt 5 ]; do
    if [ "$SERVICE_KIND" = "systemd user scope" ]; then
        if systemctl --user is-active --quiet idlescreen.service 2>/dev/null; then
            _ready=1
            break
        fi
    elif "${TARGET_BIN}" status 2>/dev/null | grep -qi "active"; then
        _ready=1
        break
    fi
    sleep 1
    _i=$((_i + 1))
done

if [ "$_ready" -eq 1 ]; then
    ok "IdleScreen daemon is ${GREEN}${BOLD}active${RESET}"
else
    warn "IdleScreen daemon not confirmed active — check: ${TARGET_BIN} status"
fi

say ""
say "  ${GREEN}${BOLD}╔══════════════════════════════════════════════════════╗${RESET}"
say "  ${GREEN}${BOLD}║              ✦  INSTALL FINISHED  ✦                  ║${RESET}"
say "  ${GREEN}${BOLD}╚══════════════════════════════════════════════════════╝${RESET}"
say "  ${DIM}host${RESET}     ${OS_NAME} (${ARCH})"
say "  ${DIM}binary${RESET}   ${TARGET_BIN}"
say "  ${DIM}config${RESET}   ${CFG_DIR}/config.toml"
say "  ${DIM}daemon${RESET}   ${SERVICE_KIND} (zero symlinks)"
say ""
say "  ${BOLD}Quick start${RESET}"
say "    ${CYAN}idlescreen tui${RESET}            interactive settings dashboard"
say "    ${CYAN}idlescreen status${RESET}         view screensaver daemon status"
say "    ${CYAN}idlescreen toggle${RESET}         toggle standby stay-awake mode"
say "    ${CYAN}idlescreen preview ascii${RESET}  preview screensaver"
say ""
