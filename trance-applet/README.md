# trance-applet

Optional **COSMIC Desktop** panel applet for the [Trance screensaver suite](https://github.com/crateria/trance).

It is **not** pulled in by default with the core `trance` package (GNOME/KDE/Hyprland users should use `trance-tui` / `trance` instead).

## Install (COSMIC only)

```bash
# after the crateria APT/DNF repo is configured
sudo apt install trance-applet    # Debian / Ubuntu / Pop
# or
sudo dnf install trance-applet    # Fedora
```

Then: panel → **Add Applet** → search **Trance**.

## What it does

- Start/stop the `trance-daemon` user service (`enable --now` on start so it survives logins)
- Toggle idle activation, timeout, active saver, render scale, FPS overlay
- Preview a saver via the daemon (D-Bus); if needed, starts the daemon first
- Last-resort preview: packaged `trance-daemon run-plugin <name>` (not an unshipped binary)
- Falls back to `~/.config/trance/config.yaml` when the daemon is offline

## Build

```bash
cargo build --release -p trance-applet
```

Packaged binary: `/usr/bin/trance-applet` (must live under `/usr/bin` for D-Bus control peer checks).

## Configuration

`~/.config/trance/config.yaml` (shared with the daemon).

## See also

- [Main README](https://github.com/crateria/trance) — install, upgrade path, `trance doctor --fix`
- Non-COSMIC UI: `trance-tui`
