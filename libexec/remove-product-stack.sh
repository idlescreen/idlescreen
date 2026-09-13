#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Remove every package the IdleScreen installer seats (except idlescreen itself,
# which is already being erased by the package manager).
#
# Invoked from the idlescreen metapackage %preun / prerm (after a short delay
# so we do not fight an in-progress dnf/apt transaction).

set -eu

# Product stack + official savers. Keep in sync with install_honesty::PRODUCT_STACK.
PKGS="
idle-cosmic
idle-tui
idle-cli
idle-savers
idle-saver-aurora
idle-saver-beams
idle-saver-bursts
idle-saver-chaos
idle-saver-cosmos
idle-saver-glyphs
idle-saver-gnats
idle-saver-hearth
idle-saver-radar
idle-saver-ripple
idle-saver-storm
idle-daemon
"

is_desktop_uid() {
    case "$1" in
        ''|*[!0-9]*) return 1 ;;
    esac
    [ "$1" -ge 1000 ]
}

# Stop/disable idle-daemon for each logged-in desktop user (mirrors idle-daemon rpm/preun).
stop_user_daemons() {
    if ! command -v loginctl >/dev/null 2>&1; then
        :
    elif ! command -v systemctl >/dev/null 2>&1; then
        :
    else
        loginctl list-users --no-legend 2>/dev/null | while read -r uid user _rest; do
            is_desktop_uid "$uid" || continue
            [ -n "$user" ] || continue
            [ -d "/run/user/$uid" ] || continue
            [ -S "/run/user/$uid/bus" ] || continue
            if command -v runuser >/dev/null 2>&1; then
                runuser -u "$user" -- env \
                    XDG_RUNTIME_DIR="/run/user/$uid" \
                    DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
                    systemctl --user stop idle-daemon.service 2>/dev/null || true
                runuser -u "$user" -- env \
                    XDG_RUNTIME_DIR="/run/user/$uid" \
                    DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
                    systemctl --user disable idle-daemon.service 2>/dev/null || true
            else
                systemctl --user --machine="${user}@" stop idle-daemon.service 2>/dev/null || true
                systemctl --user --machine="${user}@" disable idle-daemon.service 2>/dev/null || true
            fi
        done
    fi
    # Last resort: kill leftover processes if unit stop failed (orphan after unit file gone).
    if command -v pkill >/dev/null 2>&1; then
        pkill -x idle-daemon 2>/dev/null || true
        pkill -f '/usr/bin/idle-daemon' 2>/dev/null || true
    fi
}

erase_rpm() {
    to_erase=""
    for p in $PKGS; do
        if rpm -q "$p" >/dev/null 2>&1; then
            to_erase="$to_erase $p"
        fi
    done
    # shellcheck disable=SC2086
    if [ -n "$to_erase" ]; then
        # --nodeps: break circular "meta vs modules" ordering; we intentionally
        # wipe the product set when the brand package is removed.
        rpm -e --nodeps $to_erase 2>/dev/null || true
    fi
}

erase_deb() {
    to_erase=""
    for p in $PKGS; do
        if dpkg-query -W -f='${Status}\n' "$p" 2>/dev/null | grep -q 'install ok installed'; then
            to_erase="$to_erase $p"
        fi
    done
    # shellcheck disable=SC2086
    if [ -n "$to_erase" ]; then
        dpkg --remove --force-depends $to_erase 2>/dev/null || true
    fi
}

remove_repo_dropins() {
    rm -f /etc/yum.repos.d/idlescreen.repo 2>/dev/null || true
    rm -f /etc/apt/sources.list.d/idlescreen.list 2>/dev/null || true
}

remove_empty_dirs() {
    rmdir /usr/libexec/idle/screensavers 2>/dev/null || true
    rmdir /usr/libexec/idle 2>/dev/null || true
    rmdir /usr/lib/idle 2>/dev/null || true
    rmdir /usr/libexec/idlescreen 2>/dev/null || true
}

stop_user_daemons

if command -v rpm >/dev/null 2>&1 && [ -d /var/lib/rpm ]; then
    erase_rpm
fi
if command -v dpkg >/dev/null 2>&1 && [ -d /var/lib/dpkg ]; then
    erase_deb
fi
remove_repo_dropins
remove_empty_dirs

# Second pass after packages (and unit files) are gone.
stop_user_daemons

rm -f /var/tmp/idlescreen-remove-product-stack 2>/dev/null || true
exit 0
