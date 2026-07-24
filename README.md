# IdleScreen (`idle`)

IdleScreen is a modular, high-performance ambient screensaver host and idle management daemon built specifically for Wayland compositors (COSMIC, Hyprland, Sway, Wayfire, KDE Plasma Wayland).

## Architecture & Features

- **Protocol Integration**: Built on `ext-idle-notify-v1` and `zwlr_layer_shell_v1` protocols.
- **IPC Plugin Host**: Isolated process architecture hosting modular `.so` screensaver plugins.
- **Power Awareness**: Monitors `/sys/class/power_supply` to automatically pause rendering on battery power.
- **Media Inhibit**: Detects active playback from MPRIS2 media players (`org.mpris.MediaPlayer2`).
- **Smooth Ramping**: ARGB alpha opacity interpolation over initial presentation intervals.
- **Color Engine**: Configurable palettes (`synthwave`, `cyberpunk`, `neon`, `aurora`, `monokai`, `matrix`).
- **D-Bus Control**: Control via `idle` CLI or D-Bus methods on `io.github.ubermetroid.trance`.

## Installation & Build

### Prerequisites

- Rust 1.80+
- Wayland development libraries (`libwayland-dev`, `libxkbcommon-dev`, `libdbus-1-dev`)

### Building from Source

```bash
cargo build --release
```

Binaries will be placed in `target/release/`:
- `idle-daemon`: Core background daemon
- `idle`: CLI controller

## Configuration

Default configuration file path: `~/.config/idle/config.yaml`

```yaml
timeout_mins: 5
active_saver: "random"
theme: "cyberpunk"
off_timeout_mins: 15
```

## Diagnostics

Run system diagnostics and auto-repair:

```bash
idle doctor --fix
```

## License

Apache License 2.0. See LICENSE for details.