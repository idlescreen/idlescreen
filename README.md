# idlescreen

[![snip](https://img.shields.io/github/actions/workflow/status/idlescreen/idlescreen/snip.yml?label=snip&logo=shield)](https://github.com/idlescreen/idlescreen/actions/workflows/snip.yml)
[![vigil](https://img.shields.io/github/actions/workflow/status/idlescreen/idlescreen/vigil.yml?label=vigil&logo=shield)](https://github.com/idlescreen/idlescreen/actions/workflows/vigil.yml)
[![aegis](https://img.shields.io/github/actions/workflow/status/idlescreen/idlescreen/aegis.yml?label=aegis&logo=shield)](https://github.com/idlescreen/idlescreen/actions/workflows/aegis.yml)
[![proven](https://img.shields.io/github/actions/workflow/status/idlescreen/idlescreen/proven.yml?label=proven&logo=shield)](https://github.com/idlescreen/idlescreen/actions/workflows/proven.yml)
[![boneyard](https://img.shields.io/github/actions/workflow/status/idlescreen/idlescreen/boneyard.yml?label=boneyard&logo=shield)](https://github.com/idlescreen/idlescreen/actions/workflows/boneyard.yml)

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

---

<div align="center">

[![Necrometer](necrometer.svg)](https://necrometer.dev/?u=idlescreen)

</div>
