# AUDIT STAGE 3 — Mutation testing (cargo-mutants)

**Tool:** cargo-mutants **27.1.0**  
**Scope:** focused packages only (not full workspace)  
**Wall time:** ~15–20 min total (multiple `idle-ipc` iterations + one `idle-dbus` pass)  
**Flags:** `--timeout 30 --jobs 2`  
**Git:** no commit (tree left building)

## Packages exercised

| Package | Command | Mutants | Caught | Missed | Timeout | Unviable |
|---------|---------|--------:|-------:|-------:|--------:|---------:|
| **idle-ipc** (final) | `cargo mutants -p idle-ipc --timeout 30 --jobs 2` | 111 | **91** | **14** | 0 | 6 |
| **idle-ipc** (baseline first pass) | same | 111 | 58 | 47 | 0 | 6 |
| **idle-dbus** | `cargo mutants -p idle-dbus --timeout 30 --jobs 2 -- --lib` | 41 | 8 | 27 | 0 | 6 |

**idle-ipc kill rate (viable):** 91 / 105 ≈ **86.7%** after test hardening  
**idle-ipc improvement:** 47 → 14 survivors (−33), 58 → 91 caught (+33)

## idle-ipc — what was killed

Priority areas from the stage brief: **SHM validation**, **path/name sanitize**, **layout / grid math**.

### Grid / layout (`ffi_cell::validate_grid_dims`, `compute_shm_size`)

- Axis max vs oversize (`MAX_GRID_DIM` accepted, `+1` rejected).
- Cell-count cap at exact `512×512 == MAX_GRID_CELLS` and one-over (`512×513`).
- Match-guard `n <= MAX_GRID_CELLS` no longer survivable as always-true for in-range axes.

### Path / name sanitize (`path_safety`)

- SHM name: exact rest length **64** accepted; **65** / **80** rejected; charset / prefix / traversal.
- Socket path: absolute `.sock` only; nulls; `..` segments; **sun_path** length **107** accept / **108** reject (kills `||`↔`&&` on empty/len).

### SHM create/open validation (`shm::SharedMemory`)

- Invalid names on **create** and **open**.
- Size below header, tiny (`1`), oversize (`MAX+1`), exact **header**, **1 MiB** mid (kills `64*1024*1024` → `+` mutants), exact **64 MiB** max (kills `>` → `>=`).
- `open` success while owner lives (header / mid / max) so open-side bounds match create.
- `cells_mut`: bad magic, zero/correct magic, dims that exceed map.
- Accessors: `name`, `size`, live `fd > 0`, non-null `ptr`.
- Create→open cell round-trip; open after owner drop fails (unlink).

### Protocol

Pre-existing unit + proptest round-trips already catch tag / encode / decode mutants.

## Tests added / adjusted (to kill survivors)

| Area | File | Tests / changes |
|------|------|-----------------|
| Grid boundaries | `crates/idle-ipc/src/lib.rs` | `test_validate_grid_dims_boundaries` (+ tighter zeros / both axes) |
| Path sanitize | `crates/idle-ipc/src/path_safety.rs` | Exact-64 SHM rest; length 107/108 sockets; extra charset / null / `..` cases |
| SHM validation | `crates/idle-ipc/src/shm_tests.rs` | `open_rejects_*`, `create_accepts_header_mid_and_max_sizes`, `cells_mut_*`, richer `create_open_roundtrip_name` |

`cargo test -p idle-ipc --lib` → **32 passed**.

## idle-ipc residual survivors (14) — accepted for this stage

All residual MISSED mutants are in `shm.rs` and fall into classes that unit tests cannot kill without fault injection or UB observation:

1. **POSIX flag bit-ops** — `O_CREAT \| O_RDWR \| O_EXCL` and `PROT_READ \| PROT_WRITE` with `\|` → `^` still produce working mappings on Linux for the exercised paths.
2. **Syscall failure predicates** — `fd < 0`, `ftruncate < 0`, `mmap == MAP_FAILED` comparisons; no reliable way to force those failures without mocking `libc`.
3. **`Drop` guards** — null/`MAP_FAILED` / `fd >= 0` / sentinel `fd = -1` branches; double-free would be UB/leak, not a clean test assertion.

These are lower severity than name/size/magic/dim checks, which are now well covered.

## idle-dbus — notes

| Class | Outcome |
|-------|---------|
| `parse_status` / `read_bool` / `read_u32` / `read_string` | Already well tested → **caught** |
| Thin `TranceClient::{enable,disable,...}` bodies that only forward to zbus proxy | **Missed** — returning `Ok(())` / empty vectors needs a live or mock session bus |
| `daemon_available` → always true/false | **Missed** — requires controlled D-Bus name ownership |

No new dbus tests added: mock infrastructure is out of scope for a focused stage-3 slice. Status parsing (the pure logic surface) was already mutation-resistant.

## Not run (scope cap)

- Full workspace mutants
- `idle-runner` discovery/sanitize package (heavy deps: wgpu/crossterm); sanitize already has dedicated unit + proptest tests from prior stages
- `idle-upscaler` layout mutants (time budget used on ipc/dbus)

## Commands to reproduce

```bash
cargo mutants --version   # 27.1.0

cargo mutants -p idle-ipc --timeout 30 --jobs 2
cargo mutants -p idle-dbus --timeout 30 --jobs 2 -- --lib

cargo test -p idle-ipc --lib
```

## Summary

| Metric | Value |
|--------|-------|
| Focus packages | `idle-ipc`, `idle-dbus` |
| idle-ipc final | **91 caught / 14 missed / 0 timeout / 6 unviable** (of 111) |
| idle-dbus | **8 caught / 27 missed / 0 timeout / 6 unviable** (of 41) |
| Primary hardening | SHM size/name/magic/dims + path sanitize + grid caps |
| Residual risk | libc error paths, open flags XOR, Drop bookkeeping; dbus proxy stubs |

**Stage-3 mutation barrier: PASS for idle-ipc security-critical pure logic** (validate + path + size + cells). Residual survivors documented; no commit.
