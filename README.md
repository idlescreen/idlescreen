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

**IdleScreen** (`v5.0.3`) is a capability-secured screensaver and idle management suite for Linux (`359 KB` ELF) and Windows (`1.5 MB` `PE32+`), built 100% natively in **openOODA (`.oo`)** with zero external runtime dependencies.

- **`0.00%` CPU at Idle & `2.62 MiB` Flat 1,000-Frame Peak RSS**: Event-driven reactor with zero periodic wakeups at idle and per-frame OODA arena reclamation (`@oo_arena_mark` / `@oo_arena_reset_mark`), sustaining a flat `2.62 MiB` (`2,688 KB`) Peak RSS across 1,000+ continuous frames with zero heap growth.
- **Default-Deny Capability Security (`std/core/ooda.oo` + `std/sec/pqc.oo`)**: Every subsystem requires explicit capability tokens (`CAP_FS_READ`, `CAP_STDIO`) attenuated before entering the render loop, with zero `unsafe` blocks and fail-closed parser bounds (`MAX_CONFIG_BYTES = 65536`).
- **Unified Single-Binary 53-Scene Engine (`339 KB` ELF / `1.5 MB` Win64 `PE32+`)**: Renders your detected OS wordmark (`OPENOODA`, `SUPERFEDORA`, `FEDORA`, `ARCH`, `UBUNTU`, `DEBIAN`, etc.) or custom text in bevelled 16-bit block typography (`U+2580..=U+259F`) with 3-layer parallax starfields and 4×8 sub-cell sprite Scale2x compositing across **53 built-in 16-bit arcade, demoscene, and mathematical scenes** (`aurora`, `blackhole`, `breakout`, `chaos`, `cosmos`, `fireworks`, `life`, `matrix`, `metaballs`, `mmx`, `moire`, `mystify`, `pipes`, `plasma`, `radar`, `ripple`, `synthgrid`, `tetris`, `tunnel`, `zero`, and more) — zero external helper binaries or IPC overhead.
- **`18/18` Bit-Exact Golden Scene Parity & Deterministic RNG (`make det && make parity`)**: Driven by a deterministic 64-bit SplitMix64 PRNG and integer trig LUTs (`--seed <u64>`), producing bit-identical ANSI output across builds and platforms.
- **Interactive Tour & Scene Stepping**: Run `idlescreen preview --tour` to tour all 53 scenes in order (`[01/53]` to `[53/53]`) with an on-screen 16-bit HUD badge and live `←`/`→` (`n`/`p`/`Tab`) scene stepping.
- **Interactive TUI Director (`idlescreen tui`)**: Terminal configuration dashboard with a live animated side-by-side 16-bit mini-preview viewport on wide terminals (`>= 105` cols), 53-scene selector (`superfedora`, `ascii` 53 shuffled, `tour` 1–53 in order, or any pinned scene), clean `OS Name` vs. `Custom Text` banner toggle, `Dwell`, `FPS`, and `DPMS Power-Off` controls, live daemon/inhibitor status bar, and preview triggers.
- **First-Class openOODA & SuperFedora Integration**: Native capability-attenuated openOODA architecture (`std/core/ooda.oo` + `std/sec/pqc.oo`), plus first-class detection of openOODA and SuperFedora (`OPENOODA` / `SUPERFEDORA` arcade wordmarks and session awareness).
- **COSMIC Cage & Container Runtime Detection**: Autonomous zero-overhead detection of `CosmicCage`, `PodmanContainer`, `GenericContainer`, and bare-metal `Host` environments at daemon startup and in `idlescreen doctor`.
- **Multi-Init First Class**: Built-in `idlescreen service install` provisioning for `systemd` (user unit), `OpenRC`, `runit`, `s6`, `dinit`, and XDG autostart (`~/.config/autostart/idlescreen.desktop`).

---

## Installation

```sh
curl -fsSL https://idlescreen.github.io/install.sh | sh
```

### Native openOODA (`.oo`) Source Build

Build the capability-secured native Linux ELF (`bin/idlescreen`) and Windows PE32+ (`bin/idlescreen.exe`) binaries from source with `oodac`:

```sh
git clone https://github.com/idlescreen/idlescreen
cd idlescreen
make build && make verify && make test && make det && make parity
```

---

## Commands

```text
idlescreen daemon                       Run event-driven screensaver daemon
idlescreen status [--json]              Check active screensaver, inhibitor state, and daemon status
idlescreen preview [saver] [--tour]     Preview any of the 53 built-in scenes (use Left/Right or n/p to step scenes)
idlescreen list [--json]                List all 53 built-in 16-bit arcade & mathematical scenes
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
idlescreen --self-test                  Run built-in capability & 53-scene self-verification
idlescreen version                      Show version information (v5.0.3)
```

---

## Configuration

Located at `~/.config/idlescreen/config.toml`:

```toml
[general]
idle_enabled = true
idle_timeout_secs = 120
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

## Running in Containers & COSMIC Cages

IdleScreen automatically detects its runtime environment at daemon startup — `CosmicCage` (priority 1), `PodmanContainer`, `GenericContainer`, or `Host` — and reports it via `idlescreen doctor` (look for `[✓] Runtime Environment: <env>`, or `"runtime_env"` in `--json` output). No configuration is required.

- **COSMIC Cage (`CosmicCage`)**: Detected automatically via `COSMIC_CAGE`, `COSMIC_SANDBOX`, `container=cosmic-cage`, `XDG_SESSION_DESKTOP` / `XDG_CURRENT_DESKTOP`, `/.cosmic-cage`, or `/run/.cosmic-cage` markers (taking priority over container markers when a cage runs atop Podman/bubblewrap).
- **Podman (`PodmanContainer`)**: Detected via `/run/.containerenv`, `container=podman`, `/run/systemd/container`, or `libpod` cgroups.
- **Docker / OCI / Flatpak (`GenericContainer`)**: Detected via `/.dockerenv`, `/.flatpak-info`, `container` env, or `docker`/`containerd`/`kubepods`/`lxc` cgroups.

Run inside Podman:

```sh
podman run --rm -it \
  -e WAYLAND_DISPLAY="$WAYLAND_DISPLAY" \
  -e XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
  -v "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY:/$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" \
  -v "$HOME/.config/idlescreen:$HOME/.config/idlescreen" \
  idlescreen daemon
```

Run inside Docker:

```sh
docker run --rm -it \
  -e WAYLAND_DISPLAY="$WAYLAND_DISPLAY" \
  -e XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
  -v "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY:/$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" \
  -v "$HOME/.config/idlescreen:$HOME/.config/idlescreen" \
  idlescreen daemon
```

Verify the detected environment with:

```sh
idlescreen doctor
```

---

## License

Apache-2.0 · © 2026 IdleScreen
