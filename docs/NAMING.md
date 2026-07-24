# IdleScreen ship names (v2)

Coordinated rename from historical `trance*` package/binary names.

## User-facing packages

| Role | Package | Binary / unit | Obsoletes / Provides |
|------|---------|---------------|----------------------|
| Daemon (idle-core) | `idlescreen` | `idlescreen-daemon`, unit `idlescreen-daemon.service` | `trance` |
| CLI | `idlescreen-cli` | `idlescreen` | `trance-cli` |
| **All savers** | `idlescreen-savers` | — (hard-depends every `saver-*`) | `trance-plugins-all` |
| One saver | `saver-<name>` | `.so` under libexec | `trance-plugin-<name>` |
| TUI | `app-tui` | `app-tui` | `trance-tui` |
| **COSMIC product** | **`app-cosmic`** | applet `idlescreen-applet` | `idlescreen-applet`, `trance-applet`, `idlescreen-cosmic` |

## Product install (COSMIC)

```bash
sudo dnf install app-cosmic
# or
sudo apt install app-cosmic
```

`app-cosmic` (from [app-cosmic](https://github.com/idlescreen/app-cosmic)) **Requires**:

1. **`idlescreen`** — idle-core daemon  
2. **`idlescreen-savers`** — every official `saver-*` plugin  
3. Ships the **COSMIC panel applet**

Optional recommends: `idlescreen-cli`, `app-tui`.

## Paths

| Purpose | Canonical | Legacy (still read) |
|---------|-----------|---------------------|
| Plugins | `/usr/libexec/idlescreen/screensavers` | `/usr/libexec/trance/screensavers` |
| Config | `~/.config/idlescreen/` | `~/.config/trance/` |
| Lib helpers | `/usr/lib/idlescreen/` | `/usr/lib/trance/` |
| PID | `$XDG_RUNTIME_DIR/idlescreen-daemon.pid` | `trance-daemon.pid` |

## Transitional binaries

- `trance-daemon` → same as `idlescreen-daemon`
- `trance` → same as `idlescreen`

## Frozen for ABI

| Surface | Value |
|---------|--------|
| D-Bus service / interface | `io.github.ubermetroid.trance` |
| Plugin cdylib stem | `libscreensaver_<name>.so` |
| Rust crate names | `trance-*` (workspace internal) |

## Non-COSMIC install

```bash
sudo dnf install idlescreen idlescreen-cli idlescreen-savers
```
