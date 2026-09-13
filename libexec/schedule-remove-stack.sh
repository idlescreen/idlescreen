#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Schedule product-stack cleanup after the current package transaction ends.
# Called from idlescreen %preun / prerm on full erase (not upgrade).

set -eu

SRC="/usr/libexec/idlescreen/remove-product-stack"
DST="/var/tmp/idlescreen-remove-product-stack"

if [ ! -x "$SRC" ]; then
    exit 0
fi

cp -f "$SRC" "$DST"
chmod 755 "$DST"

# Immediate stop only (do NOT run full remove-product-stack mid-transaction).
# Full stack erase still happens after sleep (db lock).
if command -v loginctl >/dev/null 2>&1 && command -v systemctl >/dev/null 2>&1; then
    loginctl list-users --no-legend 2>/dev/null | while read -r uid user _rest; do
        case "$uid" in ''|*[!0-9]*) continue ;; esac
        [ "$uid" -ge 1000 ] || continue
        [ -n "$user" ] || continue
        [ -d "/run/user/$uid" ] || continue
        [ -S "/run/user/$uid/bus" ] || continue
        if command -v runuser >/dev/null 2>&1; then
            runuser -u "$user" -- env \
                XDG_RUNTIME_DIR="/run/user/$uid" \
                DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
                systemctl --user stop idle-daemon.service 2>/dev/null || true
        else
            systemctl --user --machine="${user}@" stop idle-daemon.service 2>/dev/null || true
        fi
    done
fi
pkill -x idle-daemon 2>/dev/null || true

run_later() {
    /bin/sh -c 'sleep 3; /var/tmp/idlescreen-remove-product-stack' &
}

if command -v systemd-run >/dev/null 2>&1; then
    systemd-run --no-block --collect \
        --unit="idlescreen-remove-stack.service" \
        --description="IdleScreen product stack cleanup after idlescreen erase" \
        /bin/sh -c 'sleep 3; /var/tmp/idlescreen-remove-product-stack' \
        2>/dev/null || run_later
else
    run_later
fi

exit 0
