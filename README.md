# IdleScreen

**IdleScreen** is a modular, high-performance ambient screensaver host and idle management suite designed for Wayland compositors (COSMIC, Hyprland, Sway, GNOME, KDE Plasma).

🌐 **Official Website:** [https://idlescreen.github.io](https://idlescreen.github.io)

---

## ⚡ Quick Install

> 📦 **Supported Linux Distributions:** **Arch**, **Debian**, **Fedora**

```bash
curl -fsSL https://idlescreen.github.io/packages/install.sh | sh
```

---

## 🎬 Official Screensaver Gallery

IdleScreen includes 10 real-time procedural GPU & TUI visual effect modules out of the box:

| Module | Description | Preview Command | Video Preview |
|--------|-------------|-----------------|---------------|
| **Beams** | Vector laser particle beams crossing in space | `idlescreen preview beams` | <video autoplay loop muted playsinline src="https://idlescreen.github.io/assets/videos/beams.mp4" width="280"></video> |
| **Cosmos** | Deep space starfield & nebula warp simulation | `idlescreen preview cosmos` | <video autoplay loop muted playsinline src="https://idlescreen.github.io/assets/videos/cosmos.mp4" width="280"></video> |
| **Bursts** | Supernova geometry & shockwave physics | `idlescreen preview bursts` | <video autoplay loop muted playsinline src="https://idlescreen.github.io/assets/videos/bursts.mp4" width="280"></video> |
| **Storm** | Particle storm with lightning displacement | `idlescreen preview storm` | <video autoplay loop muted playsinline src="https://idlescreen.github.io/assets/videos/storm.mp4" width="280"></video> |
| **Chaos** | Mathematical attractor chaos fractals | `idlescreen preview chaos` | <video autoplay loop muted playsinline src="https://idlescreen.github.io/assets/videos/chaos.mp4" width="280"></video> |
| **Hearth** | Warm ambient embers & fire simulation | `idlescreen preview hearth` | <video autoplay loop muted playsinline src="https://idlescreen.github.io/assets/videos/hearth.mp4" width="280"></video> |
| **Ripple** | Fluid wave dynamics & caustics | `idlescreen preview ripple` | <video autoplay loop muted playsinline src="https://idlescreen.github.io/assets/videos/ripple.mp4" width="280"></video> |
| **Radar** | Polar sonar sweep radar tracking | `idlescreen preview radar` | <video autoplay loop muted playsinline src="https://idlescreen.github.io/assets/videos/radar.mp4" width="280"></video> |
| **Glyphs** | Digital matrix stream character cascade | `idlescreen preview glyphs` | <video autoplay loop muted playsinline src="https://idlescreen.github.io/assets/videos/glyphs.mp4" width="280"></video> |
| **Gnats** | Swarming autonomous agent behavior | `idlescreen preview gnats` | <video autoplay loop muted playsinline src="https://idlescreen.github.io/assets/videos/gnats.mp4" width="280"></video> |

---

## ✨ Features & Architecture

- **⚡ WGPU Low-CPU Engine:** High-performance hardware acceleration with fallback software rasterization for terminal & headless hosts.
- **🪐 COSMIC DE Panel Applet:** Native System76 COSMIC desktop integration with 1-click status control.
- **📺 Universal Terminal Host:** Interactive TUI (`idlescreen tui`) for terminal power users.
- **🔋 Smart Power Management:** Automatic battery state detection and media playback inhibit listeners.

---

## 🛠️ Manual Package Installation

If you prefer to manually configure your Linux package manager:

<details>
<summary><b>Arch Linux (`makepkg`)</b></summary>

<br>

```bash
git clone https://github.com/idlescreen/packages.git
cd packages/arch
makepkg -si
```
</details>

<details>
<summary><b>Debian / Ubuntu / Pop!_OS (APT)</b></summary>

<br>

```bash
# Add Keyring & Repository
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://idlescreen.github.io/packages/apt/idlescreen-keyring.gpg | sudo tee /etc/apt/keyrings/idlescreen-keyring.gpg >/dev/null
echo "deb [signed-by=/etc/apt/keyrings/idlescreen-keyring.gpg] https://idlescreen.github.io/packages/apt stable main" | sudo tee /etc/apt/sources.list.d/idlescreen.list >/dev/null

# Update Index & Install Product
sudo apt update
sudo apt install idlescreen
```
</details>

<details>
<summary><b>Fedora / RHEL / CentOS Stream (DNF)</b></summary>

<br>

```bash
# Add DNF Repository
sudo curl -fsSL https://idlescreen.github.io/packages/rpm/idlescreen.repo -o /etc/yum.repos.d/idlescreen.repo

# Refresh Metadata & Install Product
sudo dnf check-update
sudo dnf install idlescreen
```
</details>

---

## 💻 CLI Commands

Control IdleScreen from your terminal using `idlescreen` (or short alias `idle`):

```bash
idlescreen tui            # Launch interactive terminal UI dashboard
idlescreen status         # Check daemon and active screensaver status
idlescreen trigger        # Trigger screensaver immediately
idlescreen on             # Enable screensaver engine
idlescreen off            # Disable screensaver engine
idlescreen preview <name> # Preview a specific screensaver module
idlescreen doctor         # Run system health and Wayland diagnostic check
```

---

## 📺 Terminal UI (`idlescreen tui`)

Launch the live interactive dashboard in any terminal window:

```bash
idlescreen tui
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch between Dashboard, Savers, and Settings panes |
| `Space` | Toggle screensaver engine On / Off |
| `Enter` | Trigger screensaver immediately |
| `c` | 1-Click install COSMIC DE panel applet (COSMIC DE only) |
| `q` | Quit TUI |

---

## 🔗 Links

- **Official Website:** [https://idlescreen.github.io](https://idlescreen.github.io)
- **GitHub Organization:** [github.com/idlescreen](https://github.com/idlescreen)
