# AUDIT Stage 3 — Test expansion (security & correctness seams)

**Scope:** High-value unit tests for Stage-1/2 hardening seams.  
**Constraint:** Each `.rs` file ≤250 lines; prefer existing `*_tests.rs` modules.  
**Command:** `cargo test --workspace --exclude idle-plugins-all`  
**Git:** no commit (per task).

## Test counts

| Metric | Before | After | Δ |
|--------|--------|-------|---|
| `cargo test … -- --list` (`: test$` lines) | **436** | **542** | **+106** |
| Workspace run result | (prior green) | **all pass** | — |

Notes on listing:

- `idle-daemon` ships three bins (`idle-daemon`, `idlescreen-daemon`, `trance-daemon`) that share the same unit tests; each new daemon test appears **3×** in `--list`.
- Approximate **unique** new tests: **~50** (daemon unique ~35 after de-dupe of triple-list; runner/ipc/upscaler ~15).
- Target 3:1 test:prod remains aspirational; this pass maximizes seam coverage, not fluff.

## Coverage by priority

### 1. `auth_peer` / `require_control_peer` policy

**Files:** `idle-daemon/src/dbus_server/auth_tests.rs` (expanded)

| Case | Assertion |
|------|-----------|
| Missing Unix UID | Denied for self PID and nonexistent PID |
| Same-UID alone | Denied when exe **and** comm unreadable (`u32::MAX`) |
| Wrong UID | Denied even if process exists |
| Unreadable exe path | `check_peer_exe` → `Unreadable`; `peer_comm`/`peer_exe_basename` → `None` |
| Trusted list | Full allowlist + no shell interpreters / daemon self |
| COMM length | All trusted names ≤15; `comm_matches_trusted` trims whitespace |
| `*_DBUS_TRUST_ALL` | Honored only under `debug_assertions`; ignored in release |

### 2. Discovery path injection (relative XDG / HOME)

**Files:** `idle-runner/src/discovery.rs` (tests split out), `discovery_tests.rs` (new)

| Case | Assertion |
|------|-----------|
| Relative / `..` / NUL roots | `is_safe_data_root` rejects |
| Relative `XDG_DATA_HOME` | Not injected into `get_screensaver_dirs` |
| Mixed `XDG_DATA_DIRS` | Relative/`..` segments skipped; absolute kept |
| Relative `HOME` | User trees not added; `/usr` paths remain |
| Detect | Allowlist members always present |

### 3. Layout saturating math

**Files:** `idle-daemon/src/presentation/layout_tests.rs` (expanded)

| Case | Assertion |
|------|-----------|
| `u32::MAX` width/height | `virtual_desktop` non-zero finite spans |
| `x` near `i32::MAX` | saturating add, no panic |
| Vertical stack span | Correct height |
| Negative relative coords | Cell bounds start at 0 |
| Normalize chain of `u32::MAX` widths | `x` saturates at `i32::MAX` |

### 4. Frame pacing clamps

**Files:** `frame_pacing.rs` (extract `clamp_present_fps` / `clamp_tick_hz`), `frame_pacing_tests.rs` (new), `refresh.rs` tests, `idle-upscaler` `lib_tests.rs`

| Case | Assertion |
|------|-----------|
| NaN / ±∞ / ≤0 present FPS | → 60.0; duration from clamped value finite |
| Present band | clamp to `[1, 480]` |
| Tick band | clamp to `[15, 240]`; non-finite → 60 |
| Refresh policy | single-output floor 60; min/max/primary sync sequential |
| `target_fps` / `simulation_tick_hz` | floor 60; env cap; env clamp outliers |

### 5. `kill_and_reap` / `failsafe_armed`

**Files:** `ipc_init.rs` (`kill_and_reap` `pub(crate)`), `ipc_init_tests.rs`, `ipc_lifecycle.rs` (`take_failsafe_arm`), `ipc_lifecycle_tests.rs`

| Case | Assertion |
|------|-----------|
| Running child + socket file | Kill, wait, unlink socket |
| Already-exited child | No hang; socket cleaned |
| Missing socket | Tolerated |
| `take_failsafe_arm` | Once-only; re-arm works; concurrent: exactly one winner |

No full Wayland/presentation stack required.

### 6. Sanitize `idle-saver-` prefix

**Files:** `idle-runner/src/launcher_tests.rs` (expanded)

| Case | Assertion |
|------|-----------|
| Package names | `idle-saver-beams` / `storm` / `.so` → allowlisted stems |
| Empty after strip | `idle-saver-` → `None` |
| Single strip | Double prefix only strips once |
| Allowlist | `idle-saver-beams` allowed; `idle-saver-evil` not |

### 7. SHM magic reject / `path_safety`

**Files:** `crates/idle-ipc/src/path_safety.rs`, `shm_tests.rs`

| Case | Assertion |
|------|-----------|
| SHM names | Accept daemon format + `_`; reject traversal, spaces, len>64, wrong prefix |
| Socket paths | Reject relative, `..`, NUL, len≥108; accept runtime paths |
| `create`/`open` | Reject bad names and size bounds |
| `cells_mut` | Reject bad magic / oversized dims; accept magic 0 and `SHM_MAGIC` |
| Owner drop | Unlink; reopen fails |

## Production helpers introduced (testability only)

| Helper | Location | Role |
|--------|----------|------|
| `clamp_present_fps` / `clamp_tick_hz` | `frame_pacing.rs` | Pure clamp used by `FramePacing::compute` |
| `kill_and_reap` | `ipc_init.rs` | `pub(crate)` for unit tests |
| `take_failsafe_arm` | `ipc_lifecycle.rs` | Once-gate extracted from `trigger_failsafe_once` |

No behavior change beyond extracting existing logic into named functions.

## File size law

All new/edited `.rs` files remain **≤250 lines**.

## Verification

```text
cargo test --workspace --exclude idle-plugins-all
# → all packages / bins / doctests: ok (0 failed)
```
