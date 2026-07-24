# D-Bus ABI (frozen)

Product control plane for IdleScreen live runtime. Historical names are
**intentional ABI** — do not rename without a major version and migration plan.
See also [BOUNDARIES.md](BOUNDARIES.md).

## Well-known names

| Constant | Value |
|----------|--------|
| Service | `io.github.ubermetroid.trance` |
| Interface | `io.github.ubermetroid.trance` |
| Object path | `/io/github/crateria/trance` |

Defined in `crates/trance-dbus` as `SERVICE_NAME`, `INTERFACE_NAME`, `OBJECT_PATH`.

## Clients

- `trance-cli` (this workspace)
- `trance-tui` / app-tui
- COSMIC applet / app-cosmic

## Stability

- Method and property shapes used by the above clients are **stable**.
- Adding optional methods is preferred over changing existing signatures.
- Removing or renaming bus names is a **breaking** change requiring a coordinated
  major version and client updates.

## Boundaries

D-Bus is the **product control plane**. Display presentation uses Wayland, not
D-Bus. Plugins do not speak D-Bus.
