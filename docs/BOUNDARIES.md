# IdleScreen first-principle boundaries

This document is the **locked architectural frame** for IdleScreen. All product
repos (engines, apps, savers, export tools) should respect these lanes.

Human brand: **IdleScreen**.  
GitHub org: **idlescreen**.  
Historical package/crate/binary names may remain `trance*` for install and ABI
stability. Historical D-Bus names may remain for ABI continuity. Those names do
not change the boundaries below.

## Mental model

IdleScreen's core lane is:

> Given the compositor's idle signal and the session's permission, host pure
> visual plugins and present their frames as a guest surface, with a private
> control API.

- **Above the line:** kernel, compositor, desktop environment / session.
- **On the line:** Wayland protocols, D-Bus product API, plugin FFI.
- **Below the line:** saver effect math (`update` / `draw`).

Offline export (`idle-render`) reuses the pure side (plugins + raster) and does
not require a compositor. Platform apps (`app-cosmic`, future `app-kde`, etc.)
are DE-shaped clients of the control plane, not a second display server.

## Stack lanes

```
User intent: ambient motion when away
        |
DE / session (lock, power, user systemd)     -- NOT IdleScreen core
        |
Compositor (surfaces, focus, vsync, seat)  -- NOT IdleScreen core
        |
Wayland protocols (idle-notify, layer-shell) -- BOUNDARY (we speak)
        |
idle-core (policy, plugin host, raster, D-Bus) -- WE OWN
        |
saver-* plugins (pure sim)                 -- WE DEFINE API; content in saver repos
        |
Kernel (DRM, input, Landlock, IPC)         -- NOT IdleScreen core (we use APIs)
```

## What IdleScreen owns

1. **Product idle policy** -- when to start/stop presentation given observed
   signals (compositor idle, inhibit, lock, config, preview).
2. **Plugin universe** -- `trance-api` trait/ABI, discovery, allowlist, load,
   sandbox after load, seed env for determinism.
3. **Raster of the cell grid** -- `TerminalCell` to pixels; upscale/letterbox.
4. **Product control plane** -- D-Bus API and CLI (TUI/applet are clients).
5. **Process packaging we invent** -- user unit, optional OOP plugin runner,
   package names we ship.

## What IdleScreen does not own

| Layer | Owns | We only |
|-------|------|---------|
| Kernel | DRM/KMS, drivers, scheduling | Be a client; use Landlock/seccomp as tools |
| Compositor | Surfaces, stacking, vsync, outputs | Create guest surfaces via protocols |
| DE / session | Lock screen, greeter, session lifecycle | Yield when locked; ship user unit if allowed |
| Other apps | Their windows and inhibit | Honor inhibit |
| Distro repos | System package policy | Publish into our packages host |

No direct KMS, no raw evdev product path, no replacing the DE lock screen unless
a desktop explicitly provides that integration slot.

## Protocol contracts (the thin wires)

| Wire | Role |
|------|------|
| Wayland idle-notify | Compositor tells us the user is idle |
| layer-shell (or equivalent) | We request a full-screen guest layer |
| Wayland outputs | Geometry for multi-monitor policy |
| D-Bus (product name) | Control ABI for cli / tui / applet |
| Plugin FFI + trance-api | Effect ABI for saver-* |
| Optional SHM/IPC | Isolation between daemon and plugin process |
| Landlock | Kernel-enforced FS constraint on plugin host |

If a feature is not expressible on these wires, it is almost certainly outside
our lane.

## First principles

1. **Simulation is pure; presentation is impure.**  
   `dt`, grid, RNG, plugin state are pure. When frames appear, which output, and
   vsync are impure (compositor). Offline render stays on the pure side plus
   our rasterizer.

2. **Idle is a signal, not a second presence detector.**  
   Prefer compositor idle + inhibit + lock + config. Do not fight the compositor
   with a parallel input stack.

3. **Fullscreen ambience is a guest of the session.**  
   Lock, inhibit, and DE policy win. That is security and UX, not optional polish.

4. **Plugins are content; the host is a runtime.**  
   Host: lifecycle, sandbox, display handoff, control plane.  
   Plugin: no Wayland, no D-Bus, minimal filesystem.

5. **Control plane is product ABI; display plane is OS ABI.**  
   D-Bus changes break our clients. Wayland usage changes track the ecosystem.
   Do not conflate them.

6. **Brand vs install names is packaging, not architecture.**  
   One product architecture; frozen historical names are ABI, not a second model.

## Repo placement relative to the model

| Repo pattern | Lane |
|--------------|------|
| idle-core | Runtime: daemon, API, runner, CLI |
| saver-* | Content: pure effects |
| idle-render / idle-studio | Export: pure sim + raster + encode (no compositor) |
| idle-tui | Control-plane client (terminal) |
| app-* | Platform chrome / metapackage (DE or store) |
| packages | Distribution host for Linux packages |
| idle-pro / idle-brand | Business and brand, not runtime |

## Explicit refusals

By design IdleScreen does **not** aim to be:

- a display server or compositor
- the login or lock screen (unless a DE invites that slot)
- the owner of system power/DPMS policy
- a replacement for DE settings apps
- guaranteed identical on every DE (GNOME vs wlroots is a platform difference)

## Change control

- Treat this file as architecture law for the org.
- Product PRs that cross a boundary (e.g. raw KMS, lock-screen replacement,
  compositor-in-process) require an explicit design decision and an update here.
- Related maps: [TARGET.md](TARGET.md) (repo inventory), idle-core
  `docs/BOUNDARIES.md` (engine-local copy).

Last locked: 2026-07-23.
