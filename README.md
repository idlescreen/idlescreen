# idlescreen

The front door — one binary that routes to every IdleScreen component.
Part of [IdleScreen](https://idlescreen.github.io) — modular Wayland
screensavers for Linux.

## Install

```sh
curl -fsSL https://idlescreen.github.io/install.sh | sh
```

## Commands

```text
idlescreen install <comp>...   install components ('all' for everything)
idlescreen doctor [--fix]      diagnostics + repair
idlescreen preview <name>      fullscreen saver preview
idlescreen update              upgrade IdleScreen packages
idlescreen tui                 runtime configuration
```

Component verbs (`components`, `remove`, `versions`) and everything else
route through to `idle-cli` — `idlescreen --help` lists the full surface.

## License

Apache-2.0 · © 2026 IdleScreen
