# AUDIT Stage 1 — Logic & Crypto Skeptic

**Scope:** Auth/trust, plugin load path safety, integer overflow in layout/pacing, RNG misuse, config/discovery path injection.  
**Rule:** Zero-complaint — fix or PASS.  
**Constraints:** Preserve D-Bus wire ABI and FFI `trance_api_version`. No git commit.  
**Date:** 2026-07-23  

## Test gates

```text
cargo test -p idle-runner --lib   → 44 passed
cargo test -p idle-daemon         → 82 passed (×3 bin targets)
cargo test -p idle-api --lib rng  → 12 passed
```

---

## P0 — Fixed

### 1. D-Bus same-UID allowlist bypass (`auth`)

**Finding:** When `/proc/<pid>/exe` was unreadable (Yama / systemd hardening), `is_trusted_control_peer` accepted **any** peer with the same Unix UID. On a session bus that undoes the trusted-peer allowlist: any compromised same-user process could call control methods (enable/disable, timeout, preview, saver switch).

**Fix:**
- Always require peer UID == our euid; **deny if UID missing**.
- On `Unreadable` exe: require `/proc/<pid>/comm` match against `TRUSTED_CONTROL_PEERS` (15-char kernel truncation aware). Pure same-UID alone is **denied**.
- On `Trusted` path: double `check_peer_exe` re-check to narrow PID TOCTOU.
- Debug-only trust hatch accepts `IDLE_DBUS_TRUST_ALL=1` as well as legacy `TRANCE_DBUS_TRUST_ALL=1`.

**Files:**
- `idle-daemon/src/dbus_server/auth.rs` (orchestration)
- `idle-daemon/src/dbus_server/auth_peer.rs` (exe/comm inspection; split for ≤250-line law)
- `idle-daemon/src/dbus_server/auth_tests.rs`

### 2. Grid allocation overflow (`ipc_session` / `plugin_session`)

**Finding:** `vec![…; cols * rows]` used wrapping multiplication. Hostile or buggy dimensions could OOM-panic or under-allocate.

**Fix:** `checked_mul` before allocate; IPC path returns `Err` / skips frame on overflow.

**Files:**
- `idle-daemon/src/presentation/ipc_session.rs`
- `idle-runner/src/plugin_session/mod.rs`

---

## P1 — Fixed

### 3. Layout / virtual-desktop integer wrap

**Finding:** `layout.width as i32` and `x + width` could wrap for huge extents; negative relative coords cast to `usize` wrapped to huge indices; `max_x - min_x` could underflow before cast.

**Fix:**
- `extent_i32` / `nonneg_usize` helpers
- `saturating_add` / `saturating_sub` for spans
- Division denominators clamped to ≥1

**Files:**
- `idle-daemon/src/presentation/layout.rs`
- `idle-daemon/src/presentation/layout_tests.rs`

### 4. Frame pacing / simulation rate non-finite Hz

**Finding:** `Duration::from_secs_f32(1.0 / fps)` and plugin physics duration assumed finite positive Hz. NaN/inf (or a future `target_fps` regression to 0) yields zero-duration busy loops.

**Fix:** Clamp present FPS to `[1, 480]` and tick Hz to `[15, 240]` with finite guards in `FramePacing::compute`; clamp `set_simulation_rate` similarly in-process.

**Files:**
- `idle-daemon/src/presentation/frame_pacing.rs`
- `idle-runner/src/plugin_session/mod.rs`

### 5. Env path injection in plugin discovery & config roots

**Finding:** `XDG_DATA_DIRS` / `XDG_DATA_HOME` / `HOME` / `XDG_CONFIG_HOME` accepted relative paths and components with `..`, expanding the trusted plugin/config search surface before canonicalize.

**Fix:** `is_safe_data_root` / `is_safe_config_root` — absolute only, no `..`, no NUL.

**Files:**
- `idle-runner/src/discovery.rs`
- `idle-daemon/src/config.rs`

### 6. Dev plugin env alias (release escape hatch)

**Finding:** Only `TRANCE_DEV_PLUGINS=1` enabled release-time dev trees; rebrand env missing. Behavior still gated by `LaunchMode::Preview` + allowlist + trust roots.

**Fix:** Accept `IDLE_DEV_PLUGINS=1` **or** `TRANCE_DEV_PLUGINS=1` in release; debug builds unchanged.

**Files:**
- `idle-runner/src/launcher_resolve.rs`
- `idle-runner/src/launcher_tests.rs`

---

## P2 — Fixed / documented

### 7. LcgRng security documentation

**Finding:** LCG is used only for visual/deterministic export paths; not for auth. Risk is misuse by future callers.

**Fix:** Doc comment on `LcgRng`: **not cryptographically secure**; never for tokens/nonces/session IDs.

**File:** `idle-api/src/rng.rs`

### 8. Residual notes (PASS with residual risk)

| Item | Status | Notes |
|------|--------|-------|
| D-Bus PID TOCTOU | Mitigated, not eliminated | Double exe check + UID + (comm\|path) reduce window; full fix needs pidfd/credentials re-query in async path |
| `/proc/comm` spoof | Accepted residual | Attacker needs same UID and can set own comm via `prctl` / exec basename — still better than any-process same-UID; path-based trust preferred when readable |
| User-writable plugin dirs (`~/.local`) | By design | Still allowlisted name + non-world-writable + under trust root; system `/usr` prefers first |
| Session-bus same-user model | By design | Control clients are same-user; auth is defense-in-depth vs other same-user apps |
| `trance_api_version` / D-Bus names | Unchanged | No ABI edits |

---

## Changes summary (code)

| Area | Change |
|------|--------|
| Auth | Same-UID+comm fallback; deny missing UID; double exe check; `IDLE_DBUS_TRUST_ALL` |
| Plugins | `IDLE_DEV_PLUGINS`; safe XDG roots; allowlist/`idle-saver-` strip unchanged & tested |
| Overflow | Layout saturating math; grid `checked_mul`; FPS/Hz clamps |
| Config | Safe config root validation |
| RNG | Security-boundary docs only |

## ABI / product constraints

- D-Bus well-known name, interface, object path: **untouched**
- FFI symbol `trance_api_version`: **untouched**
- Historical peer basenames (`trance`, `trance-tui`, …) retained in allowlist
- File split only for AGENT.md 250-line law (`auth_peer.rs`)

## Residual backlog (not blocking Stage 1)

1. Optional: re-fetch D-Bus credentials after path check (async) for stronger TOCTOU close.
2. Optional: prefer `pidfd` when bus credentials expose it (newer dbus).
3. Optional: reject group-writable plugin binaries (currently only `o+w`).
