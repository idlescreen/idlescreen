# Trance Screensaver Suite <img src="assets/icon.svg" width="48" height="48" alt="trance logo" align="right">

Trance is a modular Wayland-native screensaver system for modern Linux desktops, with first-class integration for Pop!_OS and the COSMIC Desktop environment.

---

## Package Architecture

1. **`trance` (Core)**:
   The background idle daemon (`trance-daemon`), rendering font, and stock `beams` screensaver. Starts on login. Additional effects are optional.
2. **`trance-plugins-all` (Meta)**:
   Recommends the six optional screensaver plugins. Installed by default with `trance` via apt Recommends.
3. **`trance-plugin-*` (Effects)**:
   Individual screensaver plugins. Install only the ones you want, or use `trance-plugins-all`.
4. **`trance-applet` (COSMIC Panel Applet)**:
   A native COSMIC panel applet for quick toggles, timeout adjustment, and screensaver selection.

---

## Installation

### Debian / Ubuntu / Pop!_OS (APT)

```bash
# 1. Download the repository configuration and keyring
sudo curl -fsSL https://ubermetroid.github.io/packages/apt/ubermetroid-keyring.gpg -o /etc/apt/trusted.gpg.d/ubermetroid.gpg
sudo curl -fsSL https://ubermetroid.github.io/packages/apt/ubermetroid.list -o /etc/apt/sources.list.d/ubermetroid.list

# 2. Update and install
sudo apt update && sudo apt install trance
```

*Note: A standard APT install pulls in recommended plugins (`trance-plugins-all`) and the COSMIC applet (`trance-applet`). To install only the core package and stock screensaver, run `sudo apt install --no-install-recommends trance`.*

### Fedora (DNF)

```bash
# 1. Download the repository configuration
sudo curl -fsSL https://ubermetroid.github.io/packages/rpm/ubermetroid.repo -o /etc/yum.repos.d/ubermetroid.repo

# 2. Update and install
sudo dnf check-update && sudo dnf install trance
```

---

## System Defaults

* **Background Daemon**: enabled on install
* **Default Idle Timeout**: 5 minutes
* **Default Active Screensaver**: `beams`

Configuration file:
`~/.config/ubermetroid/theme.yaml` (falls back to `~/.config/local76/theme.yaml` if it exists)

---

## CLI Controller

Trance provides a unified command-line tool `trance` (built from `trance-cli`) to manage, monitor, and troubleshoot the daemon and your configuration.

### Core Commands

| Command | Usage | Description |
|---|---|---|
| `status` | `trance status [--json]` | Show live daemon state (or minified JSON for scripting) |
| `enable` / `disable` | `trance enable`, `trance disable` | Toggle idle screensaver activation |
| `preview` | `trance preview <saver>` | Preview a screensaver immediately |
| `stop` | `trance stop` | Stop any running preview or active screensaver |
| `list` | `trance list` | List all installed screensavers |

### Advanced CLI Capabilities

* **Unified Configuration (`config`):** Get, set, or list configuration parameters directly over D-Bus:
  * `trance config list` - List all settings.
  * `trance config set <key> <val>` - Set a value (e.g. `trance config set idle_timeout_mins 10`).
  * `trance config get <key>` - Retrieve a specific setting value.
* **Interactive Mode (`interactive`):**
  * `trance interactive` - Opens a text-based console menu wizard to control the screensaver and preview savers without typing arguments.
* **Diagnostics & Troubleshooting (`doctor`):**
  * `trance doctor` - Runs a local diagnostics suite checking Wayland settings, D-Bus connection, systemd service, running PID, config parsing, and monospace fonts.
* **Sanitized Bug-Reporting (`bug-report`):**
  * `trance bug-report` - Automatically packages diagnostic info and system logs into a sanitized markdown block (scrubbing home directories/usernames) ready for GitHub issues.
* **Self-Update Checking (`self-update`):**
  * `trance self-update` - Checks the local APT package policy database to alert you of new versions in the repository and displays the upgrade commands.
* **Shell Tab-Completion (`completion`):**
  * `trance completion bash` or `trance completion zsh` - Generates shell autocomplete scripts. Run `source <(trance completion zsh)` to enable Tab-completion for commands and screensaver names.
* **Pruning and Cleanup (`clean`):**
  * `trance clean` - Sweeps away stale PID files and deletes temporary local caches when the daemon is offline.

---

## D-Bus API

The daemon exports `com.ubermetroid.Trance` on the session bus at `/com/ubermetroid/Trance` (and maintains backward compatibility by exporting `com.local76.Trance` at `/com/local76/Trance` for legacy clients).

| Method | Description |
|---|---|
| `GetStatus` | Returns live daemon state (`idle_enabled`, `presentation_active`, `session_locked`, etc.) |
| `Enable` / `Disable` | Toggle idle screensaver activation |
| `SetTimeout(minutes)` | Set idle timeout (1–240 minutes) |
| `SetSaver(name)` | Set active saver (`""` = random) |
| `ListSavers` | List installed screensaver plugins |
| `Preview(name)` | Start a saver immediately |
| `StopPreview` | Stop a running preview or idle presentation |
| `Inhibit(app, reason)` | Prevent idle activation; returns a cookie |
| `UnInhibit(cookie)` | Remove an inhibit request |

`StatusChanged` is emitted when state changes. The `trance-dbus` crate provides a blocking Rust client for applet, app, and CLI use.

Lock-screen coordination uses logind `LockedHint` — presentations stop when the session locks and do not restart until unlock.

---

## Resolution Upscaling

Trance renders screensaver plugins at a reduced simulation grid, then upscales the frames to your monitor resolution on the CPU (supporting nearest-neighbor and bilinear filtering). This makes effects chunkier and smoother at high resolutions without rewriting the plugins.

The previous `trance-gpu` crate was renamed to `trance-upscaler` in 2026. The upscaler is fully CPU-based to reduce system dependencies (removing Vulkan/OpenGL requirements) and power consumption.

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `TRANCE_RENDER_SCALE` | `0.75` | Simulation grid scale (`0.25`–`1.0`). Lower = chunkier effect, more upscale |
| `TRANCE_GPU_FILTER` | `linear` | `linear` or `nearest` upscale filter |
| `TRANCE_MAX_FPS` | `0` (auto) | Cap frame rate. `0` uses detected monitor refresh (e.g. 144 Hz) |

Frame pacing reads each monitor's Wayland refresh rate and targets that (capped by `TRANCE_MAX_FPS` when set).

---

## Wayland Integration

Trance requires a Wayland session (`WAYLAND_DISPLAY`).

* **Idle detection**: `ext-idle-notify-v1`
* **Presentation**: `zwlr_layer_shell_v1` fullscreen overlays
* **Rendering**: plugin terminal grids rasterized to pixels via DejaVu Sans Mono
* **Multi-monitor**: one layer-shell surface per output

---

## Development

```bash
cargo build --release -p trance-daemon
systemctl --user stop trance-daemon
~/.local/bin/trance-daemon daemon
```