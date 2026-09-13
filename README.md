# idlescreen

The front door — one binary that routes to every IdleScreen component.
Part of [IdleScreen](https://idlescreen.github.io) — modular Wayland
screensavers for Linux.

## Use

```sh
curl -fsSL https://idlescreen.github.io/packages/install.sh | sh
```

```text
idlescreen components            show the component catalog + install state
idlescreen <component> [args]    run a component (cli, tui, studio, cosmic)
idlescreen <verb> [args]         daemon verbs forward to idle-cli
idlescreen install <comp>...     install components ('all' for everything)
idlescreen remove <comp>...      remove components
idlescreen versions              installed version of every component
```

Anything unrecognized falls through to `idle-cli`, so `idlescreen <verb>`
keeps working exactly as before.

## Develop

Zero external deps — no sibling checkouts needed.

```sh
git clone https://github.com/idlescreen/idlescreen.git && cd idlescreen
cargo build && cargo test
```

## License

Apache-2.0 · © 2026 IdleScreen
