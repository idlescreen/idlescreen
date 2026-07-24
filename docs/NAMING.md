# IdleScreen names

## What users install

| Product | Package | Pulls |
|---------|---------|--------|
| COSMIC | **`idle-cosmic`** | `idle-daemon` + `idle-savers` + applet |
| TUI | **`idle-tui`** | `idle-daemon` |
| Studio | **`idle-studio`** | offline director (uses `render`) |

Do **not** advertise `dnf install idle` or `dnf install idle-daemon` as the product path.

## Engine packages (dependencies)

| Package | Role | Binary |
|---------|------|--------|
| `idle-daemon` | Background host | `idle-daemon` |
| `idle-cli` | Control CLI | **`idle`** |
| `idle-savers` | All effects meta | — |
| `idle-saver-*` | One effect | `.so` under `/usr/libexec/idle/screensavers/` |

## CLI

After a product install (or with `idle-cli` pulled as a recommend):

```bash
idle status
idle doctor
idle preview beams
```

## Source repo

GitHub: **[idlescreen/idle](https://github.com/idlescreen/idle)** (engine workspace).  
Not a user install unit.

## Frozen ABI

- D-Bus: `io.github.ubermetroid.trance`
- Plugin stem: `libscreensaver_<name>.so`

## Engine crates (workspace)

| Crate | Role |
|-------|------|
| `idle-api` | Plugin host/plugin shared API |
| `idle-runner` | Plugin load, raster, launcher |
| `idle-daemon` | Background host binary crate |
| `idle-cli` | CLI binary crate (`idle`) |
| `idle-dbus` | D-Bus client/types (wire names historical) |
| `idle-ipc` | SHM/UDS out-of-process IPC |
| `idle-upscaler` | CPU frame upscaling |
| `idle-plugins-all` | Meta package → ships as `idle-savers` |

External plugins that used `trance-api` path deps should switch to `idle-api`.
