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

**IdleScreen** is a high-performance, resource-efficient screensaver and idle management suite for Linux.

- **0.00% CPU at idle**: Pure event-driven epoll reactor sleeping with zero periodic wakeups.
- **Native Wayland presentation**: Hardware destination scaling via `wp_viewporter` and direct software font rasterization into `wl_shm` (<0.5ms frame time).
- **Pure in-process default**: Built-in ASCII screensaver engine with dynamic Host OS, Desktop Environment, and Linux Kernel tri-rotation mutation across 37 effects, featuring responsive multi-cell font scaling (up to 400% / 20 rows) for high-resolution laptop and fullscreen displays.
- **Multi-init first class**: Native support for systemd, OpenRC, runit, dinit, s6, and standalone supervisor.
- **Clean split**: Optional `idlescreen-extras` helper package housing 11 legacy mathematical screensavers over anonymous kernel `socketpair` and sealed `memfd_create`.

---

## Installation

```sh
curl -fsSL https://idlescreen.github.io/install.sh | sh
```

To install the optional legacy mathematical screensavers:

```sh
sudo dnf install idlescreen-extras    # Fedora / RHEL
sudo apt install idlescreen-extras    # Debian / Ubuntu
```

---

## Commands

```text
idlescreen daemon            Run event-driven screensaver daemon (or start idlescreen-daemon)
idlescreen status            Check active screensaver and daemon status
idlescreen preview [saver]   Preview screensaver in terminal (crossterm)
idlescreen list              List available built-in and extras screensavers
idlescreen set <saver>       Set active screensaver (e.g. ascii, storm, aurora)
idlescreen stop              Stop running screensaver
idlescreen doctor            Run diagnostic check on display, sockets, and configuration
idlescreen version           Show version information
```

---

## Configuration

Located at `~/.config/idlescreen/config.toml`:

```toml
[general]
idle_timeout_secs = 120
primary_output = ""       # Auto-detects largest resolution or compositor hint

[saver.ascii]
# Mode options:
#  - "cycle"  (default): Randomly mutates between OS, DE, and Kernel across effects
#  - "os":     Locks text strictly to detected Host OS (e.g., "Fedora Linux")
#  - "de":     Locks text strictly to detected Desktop Environment (e.g., "GNOME")
#  - "kernel": Locks text strictly to detected Linux Kernel (e.g., "Linux 6.13.5")
#  - "custom": Locks text to user-defined `text` string below
mode = "cycle"

# Custom text override (active when mode = "custom" or when set)
text = ""

# Visual pacing & animation
fps = 60
dwell_secs = 5.0
dissolve_duration_secs = 0.7
```

---

## License

Apache-2.0 · © 2026 IdleScreen
