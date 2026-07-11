# Trance

<img src="assets/icon.svg" width="48" height="48" alt="trance logo" align="right">

Wayland-native screensaver for Linux. A background daemon watches for idle time and shows modular effects (beams, storm, radar, and more).

Works on any Wayland desktop. Control it with the **CLI**, **TUI**, or optional **COSMIC** panel applet.

### What’s new (0.3.38)

- **Upgrades keep the daemon running** — packaging no longer leaves `trance-daemon` stopped after `dnf upgrade`
- **D-Bus control works under systemd hardening** (preview/stop no longer AccessDenied)
- **Hero polish** on all eight effects via [trance-plugins](https://github.com/crateria/trance-plugins) (soft light, depth, staggered motion, intro fade)
- CLI / applet / TUI / plugins-all version-aligned with core

---

## Install

**Idea:** add the crateria package repo once, then always:

```text
install / upgrade trance  →  enable the user daemon once
```

Packages live at [crateria/packages](https://github.com/crateria/packages) (not in distro base repos).

### 1. Add the repo (once per machine)

<details>
<summary><strong>Debian / Ubuntu / Pop!_OS</strong></summary>

APT needs a GPG key + sources line so it can trust and find the packages:

```bash
sudo mkdir -p /etc/apt/keyrings
sudo curl -fsSL https://crateria.github.io/packages/apt/crateria-keyring.gpg \
  -o /etc/apt/keyrings/crateria.gpg
echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/crateria.gpg] https://crateria.github.io/packages/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/crateria.list
sudo apt update
```

</details>

<details>
<summary><strong>Fedora</strong></summary>

```bash
sudo curl -fsSL https://crateria.github.io/packages/rpm/crateria.repo \
  -o /etc/yum.repos.d/crateria.repo
```

</details>

### 2. Install trance

```bash
# Debian / Ubuntu / Pop
sudo apt install trance

# Fedora
sudo dnf install trance
```

That installs the daemon. Recommended packages (CLI, TUI, plugins) come along unless you use `--no-install-recommends` / equivalent.

**COSMIC panel only** (skip on GNOME/KDE/Hyprland):

```bash
sudo apt install trance-applet    # or: sudo dnf install trance-applet
```

### 3. Start the daemon (once per user)

It runs as a **user** systemd service (your session / Wayland)—not a system service, and not XDG autostart:

```bash
systemctl --user enable --now trance-daemon
trance status    # confirm it’s running
```

Later, if something’s wrong after an upgrade: `trance doctor --fix`.

### Upgrades (after the repo is set up)

```bash
sudo apt update && sudo apt upgrade trance && trance doctor --fix
# or
sudo dnf upgrade trance && trance doctor --fix
```

---

## How to use

Same daemon, three front ends:

| | Command / UI | Good for |
|---|--------------|----------|
| **CLI** | `trance …` | Quick commands, scripts |
| **TUI** | `trance-tui` | Any desktop, full keyboard UI |
| **Applet** | COSMIC panel | Pop!_OS / COSMIC only |

### CLI

```bash
trance version                  # also: v, --version, -V
trance about                    # longer blurb
trance status                   # also: st
trance enable | disable         # also: on / off
trance timeout 10               # also: t 10
trance list                     # also: ls
trance saver set beams          # or: random
trance preview storm            # also: p storm
trance stop
trance doctor [--fix]          # also: doc
trance self-update              # also: update
```

Flags use GNU style: `-h` / `--help`, `-V` / `--version` (not `-help` or `-version`).  
See `trance help` for the full list (including `config`, `fps`, `scale`, …).

### TUI

```bash
trance-tui
```

| Key | Action |
|-----|--------|
| `Tab` | Settings ↔ Screensavers |
| `↑` `↓` | Navigate |
| `Space` / `Enter` | Toggle or set active saver |
| `←` `→` | Timeout / render scale |
| `p` | Preview |
| `q` | Quit |

### COSMIC applet

1. `sudo apt install trance-applet` (or dnf).
2. Panel → **Add Applet** → **Trance**.
3. Click for settings; middle-click for a quick preview.

---

## Screensavers

**beams** ships with core `trance`. The rest come via recommended `trance-plugins-all` (or install one-offs like `trance-plugin-storm`).

| Name | Effect |
|------|--------|
| beams | Spotlight cones / starfield |
| bursts | City fireworks |
| chaos | Logo glitch |
| cosmos | Accretion / singularity |
| glyphs | Matrix rain + system info |
| gnats | Firefly swarm |
| radar | Sweeping radar |
| storm | Rain, lightning, wildlife |

---

## Config

`~/.config/trance/config.yaml` — shared by CLI, TUI, and applet (via D-Bus when the daemon is up).

| Env | Meaning |
|-----|---------|
| `TRANCE_RENDER_SCALE` | `0.25`–`1.0` (lower = cheaper) |
| `TRANCE_MAX_FPS` | Cap FPS; `0` = display refresh |

---

## Links

* [packages](https://github.com/crateria/packages) · [trance-plugins](https://github.com/crateria/trance-plugins) · [SECURITY.md](SECURITY.md)

## License

[Apache-2.0](LICENSE) · Copyright 2026 crateria
