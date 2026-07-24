# Stage 1 — Memory & Unsafe Audit

**Role:** Memory & Unsafe Skeptic (Zero-Complaint Rule)  
**Scope:** `crates/idle-ipc`, `crates/wayland-*`, `idle-daemon` (SHM, waitpid, memfd, mutex/async)  
**Date:** 2026-07-23  
**Commit:** not made (per task instructions)

---

## Executive summary

Primary production risk was a **double-reap race**: a detached `waitpid` watchdog
and `std::process::Child` both reaped the OOP runner pid. That is fixed by making
`Child` the sole reaper and moving failsafe into `is_plugin_alive`.

SHM / memfd RAII was already largely sound; this pass adds SAFETY contracts,
idempotent Drop, magic validation, path-safety split for the 250-line law, and a
mutex/lock-order fix so `status` is never held across logind D-Bus probes.

---

## Findings

### P0 — Fixed

| ID | Finding | Action |
|----|---------|--------|
| P0-1 | **`waitpid` race-reap** in `ipc_init.rs`: background thread called blocking `waitpid(child_pid)` while `Child` in `IpcPluginSession` also `try_wait`/`wait`/`Drop`-reaped the same pid. Could yield ECHILD, lost exit status, and races on failsafe vs recover. | Removed watchdog `waitpid` thread. Documented single-reaper rule. Failsafe now fires once from `IpcPluginSession::is_plugin_alive` on unexpected exit (gated by `failsafe_armed`). |

### P1 — Fixed

| ID | Finding | Action |
|----|---------|--------|
| P1-1 | Missing **SAFETY** documentation on SHM/mmap/memfd/`poll`/`kill(0)` sites. | Annotated all production `unsafe` in `idle-ipc` SHM, `wayland-present` buffer, Wayland poll loops, daemon pid probe. |
| P1-2 | SHM `Drop` did not null `ptr`/`fd` after teardown (theoretical double-Drop hardness). | Null `ptr` and set `fd = -1` after `munmap`/`close`. Same for `MappedBuffer` mapping. |
| P1-3 | `cells_mut` did not reject corrupted **magic**. | Reject non-zero magic ≠ `SHM_MAGIC`. |
| P1-4 | Dead `is_memfd` field (always false) added confusion about ownership model. | Removed; documented named-SHM-only contract. |
| P1-5 | **`status` held across `is_inhibited()`** in `update_live_state` — can block on logind system D-Bus while holding daemon status mutex (latency + inversion risk vs D-Bus handlers). | Snapshot `inhibited` / `session_locked` before locking `status`. Documented order: `config` → inhibitors/logind → `status`. |
| P1-6 | Wayland overlay **pool** not destroyed after buffer create (proxy leak until connection drop). | Call `pool.destroy()` after `create_buffer`. |
| P1-7 | `SharedMemory` not `Send` despite exclusive ownership (blocks future move-only handoff). | `unsafe impl Send` with ownership rationale. |

### P2 — Documented residual / not changed this stage

| ID | Finding | Residual risk / note |
|----|---------|----------------------|
| P2-1 | Failsafe latency now up to **one presentation frame** (~16–33 ms) vs immediate `waitpid`. | Acceptable; single-reaper safety preferred. |
| P2-2 | `mutex` poison paths use `into_inner()` (not panic). Correct for daemon liveness; poisoned state may briefly be inconsistent after a panic in another thread. | Prefer no panics; no change. |
| P2-3 | D-Bus async handlers call **blocking** `Mutex` and `apply_command` (config save) on the Tokio worker. | Short critical sections; not a classic deadlock. Watch for slow disk on config save. |
| P2-4 | `InhibitorState::is_inhibited` may `block_in_place` logind when a Tokio handle exists. | Safer after P1-5 (no `status` held). Still blocks a D-Bus worker if called from async without `spawn_blocking` for the whole path — today main tick + status path own most calls. |
| P2-5 | Config watcher notify callback holds no daemon locks during notify setup; `mutate_config` takes `config` only. | No lock cycle found with status/D-Bus. |
| P2-6 | `MappedBuffer::wl_buffer` relies on Wayland proxy Drop for `destroy`. | OK with current `wayland-client`. |
| P2-7 | SHM is not `Sync`; concurrent `header_mut`/`cells_mut` from two threads is protocol-UB. | Protocol is single-writer; document for future multi-thread. |
| P2-8 | Test-only `.unwrap()` / `.expect()` remain (allowed). Production paths audited in scope use `Result` / poison recovery. | OK. |
| P2-9 | `idle-runner` plugin FFI / PAM / fullscreen `unsafe` out of primary SHM/waitpid/memfd scope. | Later stage. |

---

## Deadlock / concurrency notes

### Observed lock order (daemon)

1. `config` (short, clone-or-mutate)
2. `inhibitors` / `logind_cache` (never under `status` after this pass)
3. `status`
4. `status_emit_tx` (after status snapshot released)
5. `command_rx`, `dbus_connection` — independent, short

**Rule:** never call `is_inhibited()` while holding `status`.

### Async vs blocking

- D-Bus thread owns Tokio; main tick is plain OS threads (`config_watcher`, Wayland present/idle, presentation frame loop).
- `config_watcher` intentionally avoids Tokio so startup cannot panic on missing reactor.
- `emit_status_changes` uses `recv_timeout` on `std::sync::mpsc` inside async task — blocks the worker up to 200 ms; acceptable.

### OOP runner lifecycle

```
init spawn Child
  → sole reaper: Child::{try_wait, wait, Drop}
  → unexpected exit: is_plugin_alive → failsafe once → recover()
  → intentional stop: expected_stop=true before kill/wait
```

---

## Files changed

| Path | Change |
|------|--------|
| `/tmp/idle-audit-graph/crates/idle-ipc/src/shm.rs` | SAFETY, Send, Drop hardening, magic check; path helpers extracted |
| `/tmp/idle-audit-graph/crates/idle-ipc/src/path_safety.rs` | **New** — SHM name + UDS path validation (+ unit tests) |
| `/tmp/idle-audit-graph/crates/idle-ipc/src/shm_tests.rs` | Slimmed; added `cells_mut_rejects_bad_magic` |
| `/tmp/idle-audit-graph/crates/idle-ipc/src/lib.rs` | Export `path_safety` |
| `/tmp/idle-audit-graph/crates/wayland-present/src/overlay/buffer.rs` | SAFETY, Drop null-out, `pool.destroy()` |
| `/tmp/idle-audit-graph/crates/wayland-present/src/overlay/thread.rs` | `poll` SAFETY |
| `/tmp/idle-audit-graph/crates/wayland-idle/src/wayland/thread.rs` | `poll` SAFETY |
| `/tmp/idle-audit-graph/idle-daemon/src/presentation/ipc_init.rs` | Removed `waitpid` watchdog; sole reaper = `Child` |
| `/tmp/idle-audit-graph/idle-daemon/src/presentation/ipc_lifecycle.rs` | Failsafe-once on unexpected death; single-reaper docs |
| `/tmp/idle-audit-graph/idle-daemon/src/presentation/ipc_session.rs` | `failsafe_armed`; SAFETY on SHM; init API |
| `/tmp/idle-audit-graph/idle-daemon/src/ipc_runner.rs` | SAFETY on SHM cell/header writes |
| `/tmp/idle-audit-graph/idle-daemon/src/controller/status.rs` | Inhibit probe before `status` lock |
| `/tmp/idle-audit-graph/idle-daemon/src/daemon/mod.rs` | `kill(0)` SAFETY for pidfile probe |

Line limits: all touched `.rs` files ≤ 250 lines (`shm.rs` = 242).

---

## Verification

```text
CARGO_TARGET_DIR=/tmp/idle-cargo-target CARGO_INCREMENTAL=0 \
  cargo test -p idle-ipc -p idle-dbus --quiet
# idle-ipc: 22 passed; idle-dbus: 13 passed

CARGO_TARGET_DIR=/tmp/idle-cargo-target CARGO_INCREMENTAL=0 \
  cargo check -p idle-daemon 2>&1 | tail -30
# Finished `dev` profile
```

(Host had `/tmp` disk quota pressure; tests used a clean `CARGO_TARGET_DIR` under `/tmp/idle-cargo-target`.)

---

## Residual risk (accept for later stages)

1. **Failsafe timing** depends on the presentation frame loop calling `is_plugin_alive` (not a kernel wait). If presentation is stuck before that check, failsafe delays.
2. **Logind inhibit** still does blocking system D-Bus; consider full `spawn_blocking` if it ever sits on the hot D-Bus path under `status`.
3. **Plugin FFI / PAM / env set_var** in other crates still carry their own `unsafe` inventory (out of this stage’s SHM/memfd/waitpid focus).
4. **ABI frozen:** no changes to D-Bus names, object path, `trance_api_version`, or `libscreensaver_*.so` layout.
