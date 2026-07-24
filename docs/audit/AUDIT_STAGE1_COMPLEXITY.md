# AUDIT STAGE 1 — Complexity / file-length enforcement

**Workspace:** `/tmp/idle-audit-graph`  
**Rule:** max **250** lines per `.rs` file (exclude `target/`); reduce cyclomatic density on ≥200-line complex units by extracting helpers/modules.  
**Git:** no commit (per task).

## Verdict

**PASS** — no production `.rs` file exceeds 250 lines (post-split max observed: **242** `crates/idle-ipc/src/shm.rs`).

## Baseline (pre-split inventory)

All files were already **≤250**. Largest / densest offenders targeted for helper extraction:

| Lines | File | Branch-ish score (approx) |
|------:|------|---------------------------|
| 245 | `crates/idle-upscaler/src/cpu.rs` | high loops + dual paths |
| 243 | `crates/wayland-present/src/overlay/state/overlay.rs` | dense `configure`/`update` |
| 233 | `idle-daemon/src/dbus_server/auth.rs` | high avg per-fn (path+ownership) |
| 228 | `idle-cli/src/commands.rs` | highest total branches |
| 219 | `idle-daemon/src/config.rs` | large `load` match |
| 215 | `idle-cli/src/interactive.rs` | prompt branching |
| 202 | `idle-runner/src/launcher.rs` | resolve/search branching |

## Splits performed (public API kept stable via re-exports)

### idle-daemon

| Before | After | Seam |
|--------|-------|------|
| `dbus_server/auth.rs` (monolith) | `auth.rs` + `auth_peer.rs` | Peer path/comm/ownership vs D-Bus `require_control_peer` |
| `config.rs` load match | `config.rs` + `config_parse.rs` | YAML line/key application helpers |

`main.rs` registers `mod config_parse`.  
`require_control_peer` remains the sole public auth entrypoint.

### idle-cli

| Before | After | Seam |
|--------|-------|------|
| `commands.rs` | `commands/mod.rs` + `commands/status.rs` + `commands/control.rs` | Status/version vs control subcommands |
| `interactive.rs` I/O duplication | `interactive.rs` + `interactive_io.rs` | `read_prompted_line` / index parse |

`main.rs` still `mod commands` and imports the same public handlers (`cmd_status`, `print_version`, …) via re-exports.

### idle-runner

| Before | After | Seam |
|--------|-------|------|
| `launcher.rs` resolve/search | `launcher.rs` + `launcher_resolve.rs` | Allowlist/sanitize API vs directory search |

Public surface (`resolve_saver_binary`, `sanitize_saver_name`, `ALLOWED_SAVERS`, `LaunchMode`, …) unchanged; `lib.rs` adds private `launcher_resolve`.

### idle-upscaler

| Before | After | Seam |
|--------|-------|------|
| `cpu.rs` (245) | `cpu/mod.rs` + `cpu/stretch.rs` + `cpu/letterbox.rs` + `cpu/sample.rs` | Stretch vs letterbox vs sample |

`cpu_tests.rs` still attached via `#[path = "../cpu_tests.rs"]`.  
`cpu::{StretchCache, upscale_*_into, …}` re-exported from `cpu/mod.rs` for `lib.rs`.

### wayland-present

| Before | After | Seam |
|--------|-------|------|
| `overlay/state/overlay.rs` (243) | `overlay.rs` + `overlay_frame.rs` + `overlay_geom.rs` | Create/configure/update vs frame attach vs tiling geom |

`SessionState` methods remain on the same type; `state/mod.rs` loads the new private modules.

## Post-split sizes (evidence)

```
  131 idle-daemon/src/dbus_server/auth.rs
  149 idle-daemon/src/dbus_server/auth_peer.rs
  184 idle-daemon/src/config.rs
   79 idle-daemon/src/config_parse.rs
   13 idle-cli/src/commands/mod.rs
  151 idle-cli/src/commands/status.rs
  118 idle-cli/src/commands/control.rs
  193 idle-cli/src/interactive.rs
   29 idle-cli/src/interactive_io.rs
   97 idle-runner/src/launcher.rs
  145 idle-runner/src/launcher_resolve.rs
   18 crates/idle-upscaler/src/cpu/mod.rs
  144 crates/idle-upscaler/src/cpu/stretch.rs
   71 crates/idle-upscaler/src/cpu/letterbox.rs
   75 crates/idle-upscaler/src/cpu/sample.rs
  146 crates/wayland-present/src/overlay/state/overlay.rs
  104 crates/wayland-present/src/overlay/state/overlay_frame.rs
   48 crates/wayland-present/src/overlay/state/overlay_geom.rs
```

Largest remaining (all ≤250): `shm.rs` 242, `gpu_init.rs` 240, `cell_renderer/mod.rs` 233 — left intact (already under limit; lower functional urgency than the branch-dense CLI/auth/config/launcher units).

## Build verification

```text
$ cargo fmt
$ cargo check -p idle-daemon -p idle-runner -p idle-cli 2>&1 | tail -40
```

Result (abridged):

```text
    Checking idle-runner v2.3.1 (...)
    Checking idle-daemon v2.3.1 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in …
```

Exit code **0**.  
(Also smoke-checked `idle-upscaler` and `wayland-present` after those splits.)

## Notes

- No `.unwrap()` / `.expect()` introduced in production paths of these splits.
- New modules use Apache-2.0 SPDX per project contract where newly authored.
- Concurrent workspace activity may have refined `auth.rs` / `auth_peer.rs` semantics (UID/comm fallback); the **file-length split** (`auth` vs `auth_peer`) is retained and compiles.
- No git commit performed.
