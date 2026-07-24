#!/bin/sh
# RPM %posttrans — runs after the *entire* transaction (installs + uninstalls).
#
# Purpose: after upgrades, your desktop user's idle-daemon is already on the
# new binary. You should not need to run systemctl yourself.
#
# Why posttrans (not only %post): DNF may run new %post, then old %preun. Older
# preun scripts stopped the service; this pass recovers. We only touch desktop
# UIDs (>= 1000), never root/system accounts that appear under sudo.
set -u

# shellcheck disable=SC1091
if [ -f /usr/lib/idle/user-service-lib.sh ]; then
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
    ensure_user_config_dirs() {
        _uid="$1"; _user="$2"
        if command -v runuser >/dev/null 2>&1; then
            runuser -u "$_user" -- mkdir -p "/home/$_user/.config/idle" "/home/$_user/.config/idlescreen" 2>/dev/null || true
        fi
    }
    try_restart_idle() {
        ensure_user_config_dirs "$1" "$2"
        _user_systemctl "$1" "$2" daemon-reload || true
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
fi

# Reuse try_restart_idle from the lib (same behavior as %post).
for_each_user_session try_restart_idle
exit 0
