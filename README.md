# idlescreen

The front door — one binary that routes to every IdleScreen component.
Part of [IdleScreen](https://idlescreen.github.io) — modular Wayland
screensavers for Linux.

## Install

```sh
curl -fsSL https://idlescreen.github.io/packages/install.sh | sh
```

## Commands

```text
idlescreen components            show the component catalog + install state
idlescreen <component> [args]    run a component (cli, tui, studio, cosmic)
idlescreen install <comp>...     install components ('all' for everything)
idlescreen remove <comp>...      remove components
idlescreen versions              installed version of every component
idlescreen <verb> [args]         daemon verbs forward to idle-cli
```

Anything unrecognized falls through to `idle-cli`, so `idlescreen <verb>`
keeps working exactly as before.

## License

Apache-2.0 · © 2026 IdleScreen
