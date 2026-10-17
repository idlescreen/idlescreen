# idlescreen

<div align="center">

| Security Pillar | Verification Badge |
| :--- | :---: |
| **Platform Standard** | [![secured by studio2201][b-studio]][u-home] |
| **Credential Defense** | [![snip][b-snip]][u-snip] |
| **Supply Chain Surface** | [![vigil][b-vigil]][u-vigil] |
| **Post-Quantum Cryptography** | [![aegis][b-aegis]][u-aegis] |
| **Build Provenance & SLSA** | [![proven][b-proven]][u-proven] |
| **Repository Governance** | [![boneyard][b-boneyard]][u-boneyard] |

[b-studio]: https://img.shields.io/badge/secured%20by-studio2201-2f6f5e?logo=shield
[u-home]: https://studio2201.com
[b-snip]: https://img.shields.io/badge/snip-0%20secrets-2f6f5e?logo=shield
[u-snip]: https://studio2201.com/snip
[b-vigil]: https://img.shields.io/badge/vigil-0%20dependencies-2f6f5e?logo=shield
[u-vigil]: https://studio2201.com/vigil
[b-aegis]: https://img.shields.io/badge/aegis-PQC%20compliant-2f6f5e?logo=shield
[u-aegis]: https://studio2201.com/aegis
[b-proven]: https://img.shields.io/badge/proven-ML--DSA--65%20verified-2f6f5e?logo=shield
[u-proven]: https://studio2201.com/proven
[b-boneyard]: https://img.shields.io/badge/boneyard-maintained-2f6f5e?logo=shield
[u-boneyard]: https://studio2201.com/boneyard

</div>

**IdleScreen** (`v4.5.1`) is a high-performance, zero-overhead Wayland screensaver and idle management suite for Linux.

- **0.00% CPU at idle**: Pure event-driven reactor (`epoll` / `tokio`) sleeping with zero periodic wakeups and a ~2–3 MB resident memory footprint.
- **1x Native Grid GPU Scaling (`wp_viewporter`)**: Rasterizes the character/pixel canvas at 1x native grid resolution into `wl_shm` (`<0.5 ms` frame time) and offloads fullscreen 4K/HiDPI upscaling directly to the GPU/display controller via Wayland `wp_viewporter`.
- **Dynamic 60–144 Hz refresh rate matching**: Queries Wayland `wl_output` display modes (60 Hz laptops $\to$ 60 FPS, 144 Hz+ desktops $\to$ 144 FPS) with delta-time-synchronized physics.
- **Unified Single-Binary 52-Scene 16-Bit Arcade Engine**: Renders your detected OS wordmark (`SUPERFEDORA`, `FEDORA`, `ARCH`, `UBUNTU`, `DEBIAN`, etc.) or custom text in bevelled 16-bit block typography (`U+2580..=U+259F`) with parallax starfields, 4×8 sub-cell sprite Scale2x compositing, and 400% responsive scaling across **52 built-in 16-bit arcade, demoscene, and mathematical scenes** (`aurora`, `breakout`, `chaos`, `cosmos`, `fireworks`, `life`, `matrix`, `metaballs`, `mmx`, `mystify`, `pipes`, `plasma`, `radar`, `ripple`, `synthgrid`, `tetris`, `tunnel`, `zero`, and more) — zero external helper binaries or IPC overhead.
- **Hardware DPMS Power-Off (`zwlr-output-power-management-v1`)**: Optional `dpms_timeout_secs` puts displays into hardware DPMS standby after the screensaver runs (`0%` CPU/GPU while sleeping) and wakes outputs immediately upon user input or dismissal.
- **Interactive Tour & Scene Stepping**: Run `idlescreen preview --tour` to tour all 52 scenes in order (`[01/52]` to `[52/52]`) with an on-screen 16-bit HUD badge and live `←`/`→` (`n`/`p`/`Tab`) scene stepping.
- **Overhauled TUI Dashboard with Live Side-by-Side Mini-Preview (`idlescreen tui`)**: Interactive terminal director with a live animated side-by-side 16-bit mini-preview viewport on wide terminals (`>= 105` cols), 52-scene selector (`superfedora`, `ascii` 52 shuffled, `tour` 1–52 in order, or any pinned scene), clean `OS Name` vs. `Custom Text` banner toggle, `Dwell`, `FPS`, and `DPMS Power-Off` controls, live daemon/inhibitor status bar, `[T]` standby toggle, and `[P]` Terminal / `[F]` Fullscreen preview triggers.
- **First-Class SuperFedora Integration**: Native detection of SuperFedora (`SUPERFEDORA` arcade banner, Quickshell/Hyprland session awareness, and zero-conflict idle hand-off).
- **Universal Linux OS, Desktop & Compositor Support**: Works across Wayland compositors via `wlr-layer-shell` + `ext-idle-notify-v1` (Hyprland, Sway, COSMIC, KDE Plasma, River, Wayfire, Labwc, Niri) plus native **GNOME Mutter** (`org.gnome.Mutter.IdleMonitor`) idle tracking.
- **Multi-Init First Class**: Built-in `idlescreen service install` provisioning for `systemd` (user unit), `OpenRC`, `runit`, `s6`, `dinit`, and XDG autostart (`~/.config/autostart/idlescreen.desktop`).

---

## Installation

```sh
curl -fsSL https://idlescreen.github.io/install.sh | sh
```

### Hardware-Tuned Source Build

Build from source with native CPU SIMD target optimizations (`target-cpu=native`):

```sh
git clone https://github.com/idlescreen/idlescreen
cd idlescreen
RUSTFLAGS="-C target-cpu=native" cargo build --release --workspace
```

---

## Commands

```text
idlescreen daemon                       Run event-driven screensaver daemon
idlescreen status [--json]              Check active screensaver, inhibitor state, and daemon status
idlescreen preview [saver] [--tour]     Preview any of the 52 built-in scenes (use Left/Right or n/p to step scenes)
idlescreen list [--json]                List all 52 built-in 16-bit arcade & mathematical scenes
idlescreen set <saver>                  Set active screensaver (e.g. ascii, tour, superfedora, mmx, zero, aurora, plasma, tetris)
idlescreen text [text | --clear]        Set custom arcade banner text or reset (--clear) to clean OS-only banner
idlescreen tui                          Launch interactive terminal configuration director
idlescreen start                        Trigger the configured screensaver immediately
idlescreen stop                         Deactivate the currently running screensaver
idlescreen inhibit [-- <cmd>...]        Hold an idle inhibitor while a command or task executes
idlescreen inhibitors [--json]          Inspect active D-Bus / portal / audio idle inhibitors
idlescreen service install [--init <X>] Install user service for systemd, openrc, runit, s6, dinit, or xdg
idlescreen doctor [--fix] [--json]      Diagnose Wayland protocols, sockets, and service state (with auto-remediation via --fix)
idlescreen config                       Display configuration path and active TOML settings
idlescreen version                      Show version information (v4.5.1)
```

---

## Configuration

Located at `~/.config/idlescreen/config.toml`:

```toml
[general]
idle_enabled = true
idle_timeout_secs = 300
dpms_timeout_secs = 0     # 0 = Off (never power off display), or >0 seconds after screensaver starts

[saver]
active = "ascii"

[saver.ascii]
# Clean OS-only banner by default ("SUPERFEDORA", "FEDORA", "ARCH", "DEBIAN", etc.)
# Set mode = "custom" and provide `text` (or run `idlescreen text "MY TEXT"`) for custom wordmarks.
targets = ["os"]
mode = "os"
text = ""

# Visual pacing & animation
fps = 0                   # 0 = auto-detect monitor refresh rate (60 Hz -> 60 FPS, 144 Hz -> 144 FPS)
dwell_secs = 5.0
```

---

## License

Apache-2.0 · © 2026 IdleScreen
