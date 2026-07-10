# Trance

<img src="assets/icon.svg" width="48" height="48" alt="trance logo" align="right">

Wayland-native screensaver for Linux. A background daemon watches for idle time and shows modular effects (beams, storm, radar, and more).

Works on any Wayland desktop. Control it with the **CLI**, **TUI**, or optional **COSMIC** panel applet.

---

## Install

**Idea:** add the UberMetroid package repo once, then always:

```text
install / upgrade trance  →  enable the user daemon once
```

Packages live at [UberMetroid/packages](https://github.com/UberMetroid/packages) (not in distro base repos).

### 1. Add the repo (once per machine)

<details>
<summary><strong>Debian / Ubuntu / Pop!_OS</strong></summary>

APT needs a GPG key + sources line so it can trust and find the packages:

```bash
sudo mkdir -p /etc/apt/keyrings
sudo curl -fsSL https://ubermetroid.github.io/packages/apt/ubermetroid-keyring.gpg \
  -o /etc/apt/keyrings/ubermetroid.gpg
echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/ubermetroid.gpg] https://ubermetroid.github.io/packages/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/ubermetroid.list
sudo apt update
```

</details>

<details>
<summary><strong>Fedora</strong></summary>

```bash
sudo curl -fsSL https://ubermetroid.github.io/packages/rpm/ubermetroid.repo \
  -o /etc/yum.repos.d/ubermetroid.repo
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
trance version                  # or: trance --version / trance -V
trance about                    # version + short project info
trance status
trance enable | disable
trance timeout 10
trance list
trance saver set beams          # or: random
trance preview storm
trance stop
trance doctor [--fix]
```

See also: `trance help` (`config`, `fps-overlay`, `render-scale`, `completion`, …).

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

* [packages](https://github.com/UberMetroid/packages) · [trance-plugins](https://github.com/UberMetroid/trance-plugins) · [SECURITY.md](SECURITY.md)

## License

[Apache-2.0](LICENSE) · Copyright 2026 UberMetroid
