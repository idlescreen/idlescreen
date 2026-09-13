# idlescreen

The IdleScreen front door — a single router binary that dispatches to every
component in the ecosystem.

```text
idlescreen <component> [args]   run a component (cli, tui, studio, cosmic)
idlescreen <verb> [args]        daemon commands → idle-cli (preview, stop, list, …)
idlescreen components           show the component catalog
idlescreen install <comp>...    install components ('all' for everything)
idlescreen remove <comp>...     remove components
idlescreen versions             installed versions of every component
```

Anything the router doesn't recognize is forwarded to `idle-cli`, so every
existing `idlescreen <verb>` invocation keeps working.

## Components

| Name      | Package        | Kind                             |
|-----------|----------------|----------------------------------|
| `cli`     | `idle-cli`     | runnable — protocol commands     |
| `tui`     | `idle-tui`     | runnable — config TUI            |
| `studio`  | `idle-studio`  | runnable — render queue TUI      |
| `cosmic`  | `idle-cosmic`  | runnable — COSMIC panel applet   |
| `savers`  | `idle-savers`  | managed — all official plugins   |
| `runtime` | `idle-daemon`  | managed — daemon + plugin host   |

Runnable components exec their binary with any extra arguments. Managed
components report install state (`idlescreen runtime` shows whether the
daemon is running; `idlescreen savers` lists installed plugins).

Aliases: `daemon`/`idle` → `runtime`, `plugins` → `savers`.

## Repository map

This is the router. The pieces live in their own repositories:

- [`cli`](https://github.com/idlescreen/cli) — protocol commands
- [`runtime`](https://github.com/idlescreen/runtime) — daemon, API, runner, D-Bus
- [`savers`](https://github.com/idlescreen/savers) — official screensavers
- [`studio`](https://github.com/idlescreen/studio) — render engine + TUI
- [`tui`](https://github.com/idlescreen/tui) — configuration TUI
- [`cosmic`](https://github.com/idlescreen/cosmic) — COSMIC applet
- [`packages`](https://github.com/idlescreen/packages) — signed APT/RPM channel

## Install

```sh
curl -sSL https://idlescreen.github.io/install.sh | sh
```

Or from the package channel directly:

```sh
sudo dnf install idlescreen        # Fedora/RHEL
sudo apt install idlescreen        # Debian/Ubuntu
```

## License

Apache-2.0
