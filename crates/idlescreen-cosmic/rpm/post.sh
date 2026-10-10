#!/bin/sh
# RPM %post for idle-cosmic — strip pre-rename applet leftovers.
# Old local/dev installs used com.system76.CosmicAppletIdle (Name=IdleScreen),
# which shows as a second applet next to io.github.idlescreen.CosmicApplet.
set -u

rm -f /usr/share/applications/com.system76.CosmicAppletIdle.desktop 2>/dev/null || true
rm -f /usr/share/icons/hicolor/scalable/apps/com.system76.CosmicAppletIdle-symbolic.svg 2>/dev/null || true
rm -f /usr/share/icons/hicolor/scalable/status/com.system76.CosmicAppletIdle-symbolic.svg 2>/dev/null || true
# Pre-2.5.2 idle-cli shipped a second app-launcher entry (same Name=IdleScreen as idle-tui).
rm -f /usr/share/applications/idlescreen.desktop 2>/dev/null || true

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications 2>/dev/null || true
fi

exit 0
