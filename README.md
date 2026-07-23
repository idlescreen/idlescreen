# IdleScreen

Core repository: [`idle-core`](https://github.com/idlescreen/idle-core).

[![CI](https://github.com/idlescreen/idle-core/actions/workflows/ci.yml/badge.svg)](https://github.com/idlescreen/idle-core/actions/workflows/ci.yml)
[![Security](https://img.shields.io/badge/security-private%20reporting-blue)](https://github.com/idlescreen/idle-core/security/advisories)

Modular Wayland-native screensaver and ambient display daemon for Linux, written in Rust.

| | |
|---|---|
| Brand | [idlescreen/brand](https://github.com/idlescreen/idle-brand) |
| Packages | [idlescreen.github.io/packages](https://idlescreen.github.io/packages/) |
| Org | [idlescreen](https://github.com/idlescreen) |
| Plugins | [official plugins](https://github.com/orgs/idlescreen/repositories?q=saver-) |
| Optional applet | [idlescreen/app-cosmic](https://github.com/idlescreen/idle-core-applet) |

## Install (native packages)

### Debian / Ubuntu / Pop!_OS

```bash
sudo mkdir -p /etc/apt/keyrings
sudo curl -fsSL https://idlescreen.github.io/packages/apt/crateria-keyring.gpg \
  -o /etc/apt/keyrings/idlescreen.gpg
echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/idlescreen.gpg] https://idlescreen.github.io/packages/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/idlescreen.list
sudo apt update && sudo apt install trance
```

### Fedora

```bash
sudo curl -fsSL https://idlescreen.github.io/packages/rpm/crateria.repo \
  -o /etc/yum.repos.d/idlescreen.repo
sudo dnf install trance
```

Keyring and repository drop-in filenames on the package host may still use a
historical `crateria-*` prefix; the public host is **idlescreen.github.io**.
Shipped package and binary names remain `trance` / `trance-*` for install and
API stability.

Optional packages: `trance-plugins-all`, `trance-cli` (TUI: [idle-tui](https://github.com/idlescreen/idle-tui)). COSMIC panel
users can install [app-cosmic](https://github.com/idlescreen/idle-core-applet)
separately.

## Build from source

```bash
git clone https://github.com/idlescreen/idle-core.git
cd idlescreen
cargo build --release -p trance-daemon -p trance-cli
```

System dependencies (Debian/Ubuntu): `libdbus-1-dev libwayland-dev libxkbcommon-dev libssl-dev libpam0g-dev pkg-config`

Checks (mirrors CI on `master`):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p trance-api -p trance-dbus -p trance-ipc -p trance-daemon
```

An optional multi-stage Alpine `Dockerfile` builds release binaries for
containerized tooling. Desktop install prefers native packages.

## Releases

1. Tag `vX.Y.Z` on `master`.
2. The Release workflow builds `.deb` / `.rpm` assets and publishes a GitHub Release.
3. When `IDLESCREEN_PACKAGES_DISPATCH_TOKEN` is set, the workflow sends
   `repository_dispatch` `new_release` to [idlescreen/packages](https://github.com/idlescreen/packages)
   for signing and Pages index update.

## Environment configuration

| Variable | Description | Default |
| :--- | :--- | :---: |
| `TRANCE_IDLE_TIMEOUT_MINS` | Idle minutes before screensaver | `10` |
| `TRANCE_ACTIVE_SAVER` | Active plugin name | `beams` |
| `TRANCE_SHOW_FPS` | FPS overlay | `false` |
| `LOG_LEVEL` | Tracing filter | `info` |

## Administration CLI

```bash
trance-cli status
trance-cli enable | disable
trance-cli preview <plugin>
```

## License

Apache-2.0. See [LICENSE](LICENSE).

## Architecture boundaries

IdleScreen is a **Wayland client and plugin host**, not a compositor or lock
screen. The locked first-principle frame (kernel, compositor, DE, control plane,
saver content) lives in [docs/BOUNDARIES.md](docs/BOUNDARIES.md).
