#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# RPM %postun — $1 is count remaining after this transaction (0 = full erase).
if [ "${1:-0}" -eq 0 ]; then
    # 1. Clean repo dropins and keys
    rm -f /etc/yum.repos.d/idlescreen.repo 2>/dev/null || true
    rm -f /etc/pki/rpm-gpg/idlescreen-key.gpg /etc/pki/rpm-gpg/RPM-GPG-KEY-idlescreen* 2>/dev/null || true

    # 2. Clean system configuration
    rm -rf /etc/xdg/idlescreen /etc/idlescreen /etc/idle 2>/dev/null || true

    # 3. Clean transient user caches only (preserve user ~/.config/idlescreen across package removals)
    _homes=""
    if command -v getent >/dev/null 2>&1; then
        _homes=$(getent passwd 2>/dev/null | awk -F: '$3 >= 1000 && $3 < 60000 && $6 != "" {print $6}')
    fi
    # shellcheck disable=SC2086
    for _h in $_homes ${HOME:-} /home/*; do
        [ -d "$_h" ] || continue
        rm -rf "$_h/.cache/idlescreen" 2>/dev/null || true
    done

    # 4. Clean leftover empty directories and runtime files
    rmdir /usr/libexec/idlescreen /usr/share/idlescreen 2>/dev/null || true
    rm -rf /run/user/*/idlescreen* /tmp/idlescreen* 2>/dev/null || true

    # 5. Reload user systemd daemons for logged-in sessions
    if command -v loginctl >/dev/null 2>&1 && command -v systemctl >/dev/null 2>&1; then
        loginctl list-users --no-legend 2>/dev/null | while read -r uid user _rest; do
            case "$uid" in ''|*[!0-9]*) continue ;; esac
            [ "$uid" -ge 1000 ] || continue
            if command -v runuser >/dev/null 2>&1; then
                runuser -u "$user" -- env \
                    XDG_RUNTIME_DIR="/run/user/$uid" \
                    DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
                    systemctl --user daemon-reload 2>/dev/null || true
            else
                systemctl --user --machine="${user}@" daemon-reload 2>/dev/null || true
            fi
        done
    fi
fi

exit 0
