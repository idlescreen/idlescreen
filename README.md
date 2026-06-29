# Trance Screensaver Suite <img src="assets/icon.jpg" width="48" height="48" alt="trance logo" align="right">

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

```bash
sudo apt update
sudo apt install trance
```

`trance` always installs:
- `trance-daemon`
- `fonts-dejavu-core` (monospace font for rendering)
- `trance-plugin-beams` (stock screensaver)

A typical install also pulls in (via apt Recommends):
- `trance-plugins-all` and the six optional screensaver plugins
- `trance-applet`

Core + stock saver only (no extra effects, no applet):

```bash
sudo apt install --no-install-recommends trance
```

Install specific optional plugins:

```bash
sudo apt install trance-plugin-storm trance-plugin-glyphs
```

---

## System Defaults

* **Background Daemon**: enabled on install
* **Default Idle Timeout**: 5 minutes
* **Default Active Screensaver**: `beams`

Configuration file:
`~/.config/local76/theme.yaml`

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

The daemon exports `com.local76.Trance` on the session bus at `/com/local76/Trance`.

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

## GPU Upscaling

Trance renders screensaver plugins at a reduced simulation grid, then **GPU-upscales** frames to your monitor resolution. This makes effects chunkier and smoother at high resolutions without rewriting the plugins.

**Unified API:** [wgpu](https://wgpu.rs/) over **Vulkan** (fully open source). Works with:

| Vendor | Linux driver |
|---|---|
| AMD | Mesa RADV |
| Intel | Mesa ANV |
| NVIDIA | Proprietary Vulkan driver or Nouveau |

No closed-source NVIDIA CUDA/RTX SDK is required. If Vulkan is unavailable, trance falls back to CPU bilinear upscale automatically.

**Ray tracing:** Not used today. These effects are terminal-cell simulations, not 3D scenes. Hardware RT would not map cleanly onto the plugin model and is not exposed in a portable way through wgpu on Linux. A future native-GPU plugin API could add stylized lighting, but that is a separate project.

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `TRANCE_GPU` | on | Set to `0` to force CPU upscale |
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