# AUDIT Stage 2 — Adversarial Tribunal

**Role:** Stage-2 Adversarial Verifier (Zero-Complaint)  
**Workspace:** `/tmp/idle-audit-graph`  
**Date:** 2026-07-23  
**Inputs:** `AUDIT_STAGE1_{MEMORY,LOGIC,PERF,COMPLEXITY}.md` + working-tree diffs  
**Git:** no commit (per task)

---

## Method

1. Read all four Stage-1 reports and key diffs.
2. Attack the four high-risk seams (waitpid/failsafe, auth/comm, atlas/cells_scratch, module splits).
3. `cargo test --workspace --exclude idle-plugins-all` (full green after tribunal fixes).
4. Priority: **Security > Correctness > Performance**. Max 2 debate rounds; tribunal applied fixes directly.

---

## Round 1 — Attack results

### Memory: waitpid reaper / `failsafe_armed` — **FAIL then ADJUST (not revert)**

| Claim (Stage 1) | Tribunal finding |
|-----------------|------------------|
| Remove side-thread `waitpid`; sole reaper = `Child` | **Directionally correct** — dual reaper was a real ECHILD/status race. |
| ``Child` Drop will kill+wait`` (comment in init error path) | **FALSE.** `std::process::Child` Drop neither kills nor waits. |
| Init timeout / protocol failure paths safe | **REGRESSION.** After spawn, every `return Err(...)` dropped `Child` without `kill`/`wait`. Old watchdog `waitpid` had silently reaped those pids. Long-lived daemon could leak orphans/zombies on failed connect/Init. |
| `failsafe_armed` once-gate | **PASS.** `compare_exchange(true→false)` + re-arm only in `recover()`. `expected_stop` suppresses intentional teardown. Frame loop calls `is_plugin_alive` then `recover`. |
| Plugin zombies under normal presentation | **PASS** after adjust: live path reaps via `try_wait`; teardown via `kill`+`wait` in `Drop`/`recover`; init failure via new `kill_and_reap`. |

**Tribunal action:** Added `kill_and_reap` and used it on all post-spawn failure paths in `idle-daemon/src/presentation/ipc_init.rs`. Corrected single-reaper docs in `ipc_lifecycle.rs` (do not claim Drop reaps).

**Behavioral note (accepted):** Failsafe now fires on *any* unexpected exit (including clean `exit(0)`), not only unclean status as the old watchdog did. Safer for “runner vanished while presenting”; residual: rare false-positive session lock if a runner ever exits 0 without `expected_stop`.

### Logic: auth same-UID + `/proc/comm` — **PASS with residual**

| Claim | Tribunal finding |
|-------|------------------|
| Pure same-UID fallback removed | **PASS.** `Unreadable` exe requires trusted `comm`; missing UID denied. |
| Double `check_peer_exe` on Trusted | **PASS.** Narrows TOCTOU; not a full close. |
| Tests match policy | **PASS.** `same_uid_alone_insufficient…`, `missing_peer_uid_is_denied`, `comm_matches_trusted_*`, `wrong_uid_denied…` align with new policy. |
| Can attackers spoof `/proc/comm`? | **Yes** (`prctl` / argv basename), same UID. **Residual accepted** — strictly better than any same-UID process; path trust still preferred when `exe` readable. Session-bus same-user model remains primary boundary. |
| Debug trust hatch | **PASS.** `IDLE_DBUS_TRUST_ALL` / `TRANCE_DBUS_TRUST_ALL` gated by `debug_assertions`. |

No auth reversion. No further tighten this round (pidfd / re-query credentials remain backlog).

### Perf: atlas `HashMap` / `cells_scratch` — **PASS**

| Claim | Tribunal finding |
|-------|------------------|
| `atlas_index: HashMap<char,u32>` vs linear `position` | **Correct.** `get_or_insert_atlas_char` keeps `atlas_chars` and `atlas_index` in lockstep; rebuild paints by index into `atlas_chars` (no clone). |
| Stale GPU char mapping | **No.** Hot path inserts viewport chars then rebuilds atlas before `build_gpu_cells_into`; unknown → `0xFFFFFFFF` (same as pre-change). Space → `0xFFFFFFFF`. |
| `cells_scratch` reuse | **Correct.** `clear` + reserve; unit test asserts capacity reuse and char indices. |
| Frame correctness / ABI | **Unchanged** packing and public surfaces. |

### Complexity splits: re-exports / mods — **FAIL then ADJUST**

| Split | Verdict |
|-------|---------|
| `auth` + `auth_peer` | **PASS** — `require_control_peer` stable; modules wired. |
| `config` + `config_parse` | **PASS** — `main.rs` has `mod config_parse`. |
| `commands/{mod,status,control}` | **PASS** — re-exports match `main` imports. |
| `launcher` + `launcher_resolve` | **PASS** — public resolve/sanitize surface intact. |
| `cpu/{mod,stretch,letterbox,sample}` | **BUG:** `cpu_tests.rs` used `FilterMode` via `use super::*` after nest under `cpu::`; `FilterMode` lives at crate root → compile failure in test profile. |
| overlay `overlay` / `overlay_frame` / `overlay_geom` | **PASS** — `state/mod.rs` loads private modules; `SessionState` methods remain. |
| 250-line law | **PASS** — post-tribunal max **242** (`crates/idle-ipc/src/shm.rs`). |

**Tribunal action:** `use crate::FilterMode;` in `crates/idle-upscaler/src/cpu_tests.rs`.

### Concurrent-edit hole: status lock order — **ADJUST**

Stage-1 Memory claimed “probe `is_inhibited` before `status` lock.” Working tree still evaluated `self.inhibitors.is_inhibited()` *after* `status.lock()` (Perf rewrote `apply_live_fields` without the snapshot). `is_inhibited` may block on logind system D-Bus.

**Tribunal action:** Snapshot `session_locked` + `inhibited` before acquiring `status` in `update_live_state` (`idle-daemon/src/controller/status.rs`).

---

## Round 2

Only as final application of Round-1 fixes (init reap, lock order, FilterMode import). No further agent fan-out. No original-intent tradeoff required beyond documenting failsafe-on-clean-exit as accepted hardening.

---

## Surviving changes (merge set)

| Area | Kept |
|------|------|
| Memory | Waitpid watchdog removed; failsafe-once via `is_plugin_alive` + `failsafe_armed`; SHM SAFETY / Drop null-out / magic reject / path_safety split; Wayland pool destroy + poll SAFETY |
| Logic | Same-UID + trusted-comm fallback; deny missing UID; double exe check; grid `checked_mul`; layout saturating math; FPS/Hz clamps; safe XDG/config roots; `IDLE_DEV_PLUGINS`; LcgRng security docs |
| Perf | `atlas_index` HashMap; `cells_scratch` / `build_gpu_cells_into`; caption borrow; sysinfo narrow refresh + OnceLock host info; status `apply_live_fields`; `to_map` capacity |
| Complexity | All Stage-1 file splits + re-exports (with test import fix) |
| Tribunal | Init `kill_and_reap`; status lock order; cpu_tests `FilterMode` import; accurate reaper docs |

---

## Reverted / adjusted items

| Item | Action |
|------|--------|
| Init-path child lifecycle after waitpid removal | **ADJUST** — explicit `kill_and_reap` (not full revert of single-reaper design) |
| Memory P1-5 lock order (lost under Perf edit) | **RE-APPLY** |
| `idle-upscaler` cpu split test compile break | **ADJUST** import |
| Waitpid watchdog itself | **Not reverted** |
| Auth same-UID+comm | **Not reverted** |
| Atlas / cells_scratch | **Not reverted** |

---

## Residual accepted risks

1. **`/proc/pid/comm` spoof** (same UID + `prctl`/basename) when `exe` unreadable — better than pure same-UID; prefer path trust; future: pidfd / credential re-query.
2. **Failsafe latency** up to one presentation frame (poll via `is_plugin_alive`, not kernel wait thread).
3. **Failsafe on clean unexpected exit** — stricter than old unclean-only watchdog.
4. **D-Bus PID TOCTOU** — double exe check only; not eliminated.
5. **Logind inhibit** still blocking system D-Bus (now not under `status` on the live tick path).
6. **Session-bus same-user model** — control clients are same-user by design; auth is defense-in-depth.
7. **SHM not Sync** — single-writer protocol assumed.
8. **Mutex poison `into_inner`** — daemon liveness preference.

---

## Verification

```text
CARGO_TARGET_DIR=/tmp/idle-cargo-target CARGO_INCREMENTAL=0 \
  cargo test --workspace --exclude idle-plugins-all
```

All package test binaries reported **0 failed** (including idle-daemon ×3 bins 82 each, idle-runner 44, idle-ipc 22, idle-upscaler 20, idle-dbus 13, idle-api, wayland-*, doctests).

Line law: no production `.rs` file > 250 lines (max 242).

---

## Verdict

# **MERGE READY**

Stage-1 concurrent fixes are sound after tribunal adjustments. Blocking regressions (zombie/orphan on IPC init failure, incomplete status lock order, upscaler test compile) are fixed in-tree. Remaining risks are documented and accepted.

**No git commit** performed by this stage.
