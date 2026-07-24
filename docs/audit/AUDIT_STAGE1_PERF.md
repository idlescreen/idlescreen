# AUDIT Stage 1 — Performance (hot path)

**Scope:** presentation / `plugin_session` / `cell_renderer` frame loops, sysinfo, status maps / D-Bus.  
**Constraint:** no public ABI or frame-correctness changes. No git commit.

## Findings and fixes

### 1. Hot-path clones / per-frame allocs (`cell_renderer`)

| Issue | Location | Fix |
|-------|----------|-----|
| `atlas_chars.clone()` on every atlas rebuild | `cell_renderer/atlas.rs` | Index by position (`for idx in 0..len`) so `glyph_for` can take `&mut self` without cloning the char list |
| Linear `position()` per cell for atlas char → slot | `get_or_insert_atlas_char`, `build_gpu_cells` | Maintain `HashMap<char, u32>` (`atlas_index`) for O(1) lookup on insert and GPU upload |
| `Vec<GpuCell>` allocated every GPU frame | `gpu_cells.rs` → `gpu_render.rs` | Reuse `GpuCellRenderer::cells_scratch` via `build_gpu_cells_into` |

**Frame correctness:** same GPU cell packing (bg/fg/char_idx/bold); space still maps to `0xFFFFFFFF`; unknown chars still `0xFFFFFFFF`.

### 2. Presentation / plugin_session buffer reuse

Already in good shape:

- `IpcPluginSession` / `PluginSession` keep `content_buf` + `pixel_buf` and only `mem::replace` with `Vec::with_capacity(cap)` so capacity is recycled when handing pixels to `submit_frame`.
- `raster_viewport_into` writes into those buffers in place.

Small tweak:

- SHM → grid copy in `ipc_session::draw_frame` uses `zip` instead of indexed `grid[i]` (fewer bounds checks; same conversion).

### 3. Overlays (per-frame string work)

| Issue | Fix |
|-------|-----|
| `caption_text()` cloned the caption every frame | New `idle_api::with_caption` borrows under the lock; overlays use it |
| `format!("FPS {:.1}", …)` every frame when FPS overlay on | Pre-sized `String` + `write!` (capacity 16) |

Theme load already had a 1s cache (`theme_query::load_global_theme`).

### 4. sysinfo frequency / cost

| Issue | Fix |
|-------|-----|
| `System::new_all()` + `refresh_all()` (processes, components, disks, …) | Construct with `RefreshKind` limited to CPU usage + memory; refresh only `refresh_memory` + `refresh_cpu_usage` |
| Re-read OS/kernel/hostname/CPU brand/GPU/monitors every cache miss | One-shot `StaticHostInfo` via `OnceLock` |
| 3s `SystemInfo` cache kept | Unchanged public `get_system_info() -> SystemInfo` (still clones on hit; required by return type) |

Disk summary still queried on cache miss (~3s) via existing `query_disk_drives` path — acceptable for plugin status text, not on the present loop.

### 5. Status maps / D-Bus conversion thrift

| Issue | Fix |
|-------|-----|
| Separate dirty-check + copy both called `to_string()` on savers / render_scale every tick | Single `apply_live_fields`: only `push_str` / `write!` when the value actually changes |
| Bool/int fields always rewritten | Compare-then-assign; mark dirty only on real change |
| `DaemonStatus::to_map` rehashed map growth | `HashMap::with_capacity(13)` (string fields still clone for `OwnedValue` / `'static` — required by zbus) |

**ABI:** `DaemonStatus` fields and `to_map() -> HashMap<String, OwnedValue>` unchanged.

## Tests

```text
cargo test -p idle-runner --lib --quiet
# 44 passed

Also green: idle-dbus --lib, idle-api --lib; new unit tests for
gpu_cells capacity reuse, sys_info cache smoke, render_scale_matches.
```

## Left alone (by design)

- `submit_frame(..., pixels: Vec<u8>)` ownership hand-off — would need presenter ABI change for zero-copy return.
- `ensure_buffer` Arc clones of `wgpu::Buffer` — refcount only, not a heap data copy.
- Staging map_async + mpsc per GPU readback — correctness-sensitive; not touched.
- `wayland-present` tree has pre-existing compile errors unrelated to this stage; not in scope.

## Files touched

- `idle-runner/src/cell_renderer/{atlas,mod,gpu_cells,gpu_init,gpu_render}.rs`
- `idle-runner/src/toolkit/sys_info/mod.rs`
- `idle-api/src/{caption,lib}.rs`
- `idle-daemon/src/presentation/{overlays,ipc_session}.rs`
- `idle-daemon/src/controller/status.rs`
- `crates/idle-dbus/src/status.rs`
