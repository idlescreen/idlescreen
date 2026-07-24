#!/bin/sh
# RPM %post — $1 is count of packages of this name left installed
# (1 = fresh install, 2+ = upgrade). Always best-effort.
#
# Upgrades auto-reload the user service so you do not need to run systemctl.
set -u

# shellcheck disable=SC1091
if [ -f /usr/lib/idle/user-service-lib.sh ]; then
    # Prefer the just-installed copy.
    . /usr/lib/idle/user-service-lib.sh
else
    is_desktop_uid() {
        case "$1" in ''|*[!0-9]*) return 1 ;; esac
        [ "$1" -ge 1000 ]
    }
    for_each_user_session() {
        _cb="$1"
        command -v loginctl >/dev/null 2>&1 || return 0
        command -v systemctl >/dev/null 2>&1 || return 0
        loginctl list-users --no-legend 2>/dev/null | while read -r uid user _rest; do
            is_desktop_uid "$uid" || continue
            [ -n "$user" ] || continue
            [ -d "/run/user/$uid" ] || continue
            [ -S "/run/user/$uid/bus" ] || continue
            "$_cb" "$uid" "$user" || true
        done
    }
    _user_systemctl() {
        _uid="$1"; _user="$2"; shift 2
        if command -v runuser >/dev/null 2>&1; then
            runuser -u "$_user" -- env \
                XDG_RUNTIME_DIR="/run/user/$_uid" \
                DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$_uid/bus" \
                systemctl --user "$@" 2>/dev/null && return 0
        fi
        systemctl --user --machine="${_user}@" "$@" 2>/dev/null || true
    }
    _user_is_enabled() {
        _uid="$1"; _user="$2"
        if command -v runuser >/dev/null 2>&1; then
            runuser -u "$_user" -- env \
                XDG_RUNTIME_DIR="/run/user/$_uid" \
                DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$_uid/bus" \
                systemctl --user is-enabled idle-daemon.service >/dev/null 2>&1
            return $?
        fi
        systemctl --user --machine="${_user}@" is-enabled idle-daemon.service >/dev/null 2>&1
    }
    _user_is_active() {
        _uid="$1"; _user="$2"
        if command -v runuser >/dev/null 2>&1; then
            runuser -u "$_user" -- env \
                XDG_RUNTIME_DIR="/run/user/$_uid" \
                DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$_uid/bus" \
                systemctl --user is-active idle-daemon.service >/dev/null 2>&1
            return $?
        fi
        systemctl --user --machine="${_user}@" is-active idle-daemon.service >/dev/null 2>&1
    }
    try_reload_user_units() {
        if _user_is_enabled "$1" "$2" || _user_is_active "$1" "$2"; then
            _user_systemctl "$1" "$2" daemon-reload || true
        fi
    }
    ensure_user_config_dirs() {
        _uid="$1"; _user="$2"
        if command -v runuser >/dev/null 2>&1; then
            runuser -u "$_user" -- mkdir -p "/home/$_user/.config/idle" "/home/$_user/.config/idlescreen" 2>/dev/null || true
        fi
    }
    try_restart_idle() {
        ensure_user_config_dirs "$1" "$2"
        _user_systemctl "$1" "$2" reset-failed idle-daemon.service || true
        if _user_is_enabled "$1" "$2"; then
            echo "idle: applying upgrade for $2 (user service)"
            _user_systemctl "$1" "$2" restart idle-daemon.service || true
            return 0
        fi
        if _user_is_active "$1" "$2"; then
            echo "idle: applying upgrade for $2 (running unit)"
            _user_systemctl "$1" "$2" try-restart idle-daemon.service || true
        fi
    }
    print_user_hint() {
        echo ""
        echo " 🌌 Welcome to IdleScreen!"
        echo " ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo " High-Performance GPU & Terminal Screensavers for Linux"
        echo ""
        echo " 🚀 Quick Start:"
        echo "    idlescreen tui       Launch live interactive TUI dashboard"
        echo "    idlescreen status    Check active screensaver & daemon state"
        echo "    idlescreen doctor    Run system health & Wayland check"
        echo ""
        echo " 💡 Desktop Launcher: You can also open 'IdleScreen' from your"
        echo "                       Desktop Application Launcher menu at any time!"
        echo " ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
    }
fi

# Remove legacy XDG autostart (systemd user unit is the only start path).
rm -f /etc/xdg/autostart/idle-daemon.desktop 2>/dev/null || true

for_each_user_session try_reload_user_units
for_each_user_session try_restart_idle

# Fresh install ($1 == 1): print setup hint. Upgrades stay quiet.
if [ "${1:-1}" -eq 1 ]; then
    print_user_hint
fi

exit 0
