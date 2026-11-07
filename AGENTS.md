# idlescreen: House Laws & Agent Engineering Standards (v2)

This document is the **single canonical source of truth** for all code, architecture, and system integration standards across the `idlescreen` organization. Every human contributor and AI agent must strictly follow these rules without exception.

> **Synchronization Invariant**: This document is synchronized across all `idlescreen` repositories. The canonical upstream copy lives at [`idlescreen/.github/AGENTS.md`](https://github.com/idlescreen/.github/blob/master/AGENTS.md). Automated CI gates enforce that every repository's `AGENTS.md` remains bit-for-bit identical to upstream.

---

## 1. The Page Rule (Code Layout & Sizing)

A **page** is one committed openOODA (`.oo`) file. Every page holds one idea, fits in one head, and carries its own weight. This rule is enforced by automated integration checks: red pages fail the build.

### Hard Sizing Invariants
- **16–256 Lines**: Every committed `.oo` file must be between **16 and 256 lines**, counted as `content.lines().count()` (blank lines and comments count).
- **Shim Exemption (Floor Only)**: A file is a shim when every code line is a module re-export: after dropping blank lines and comments (`//`), each remaining line must start with `import `. Shims skip the 16-line floor. The 256-line ceiling still strictly applies.
- **Directory Density (≤ 8 files)**: At most **8 `.oo` files per directory**, tests included. Crowded directories must split into functional subdirectories grouped by area.
- **Banned File Names (Name the function, not the drawer)**:
  `util.oo`, `utils.oo`, `helper.oo`, `helpers.oo`, `common.oo`, `misc.oo`, `shared.oo`, `base.oo`, `core.oo`.

### Splitting, Folding, and Naming
- **Over 256 lines**: Split along functional boundaries into a new subdirectory with a shim `anchor.oo`. One page = one verb or one wholly owned noun.
- **Under 16 lines (and not a shim)**: Fold into its closest sibling or absorb into its single caller/callee. Never pad lines to reach 16.
- **Blobs Out**: Test fixtures and static data (golden scene frames, wordmark tables) must live in dedicated files under `qa/golden/`, never inline literals.
- **Academy Headers**: Every `.oo` page opens with the four-line header (`// # Title`, `// Logline:`, `// Setup:`, `// Beats:`), enforced by `make academy`.
- **Systems & Trust Naming**:
  - The tree mirrors the running system: paths read as subsystems (`daemon/`, `wayland/`, `ascii/`, `tui/`, `cli/`).
  - Action pages lead with a verb: `admit_request.oo`, `render_frame.oo`.
  - State pages name what they own: `daemon_state.oo`, `inhibitor_table.oo`.
  - Boundary pages speak trust verbs: `verify`, `admit`, `attest`, `seal`, `enforce`, `audit`.

---

## 2. Performance Engineering & The Automated Gates

Performance in `idlescreen` refers directly to the code itself. We constantly strive for **faster, better, lighter, simpler, easier, and deeper** code:

- **Faster**: Minimal CPU cycles, low-latency event dispatch, fast frame rendering (<5ms).
- **Better**: Crash-resilient, deterministic, fails closed, verified by tests, no silent `|| true` failure swallowing.
- **Lighter**: $O(1)$ allocations in steady state, zero memory leaks, minimal binary weight, lean dependencies.
- **Simpler**: Low cognitive complexity, nesting depth $\le 3$, strict 16–256 line boundaries.
- **Easier**: Explicit domain error types (`Result[T, E]`), clean naming, decoupled blast radius.
- **Deeper**: Deep modules with high value-to-surface ratio (small, clean APIs hiding robust, optimized implementations).

### The Automated Gates (`Makefile`)
Rather than polluting source files with inline annotations, every invariant is machine-checked:
- **`make portable`**: No author-machine absolutes (`/Projects/...`), no hashed codegen names, no patching the generated runtime to disable the FS jail.
- **`make line-cap` / `file-law` / `academy` / `density` / `vocab` / `shape`**: The §1 Page Rule, file-type law, academy headers, directory density, banned vocabulary, and function shape (≤ 8 functions per page, ≤ 64 lines per function).
- **`make check`**: `oodac check` passes on every `.oo` file.
- **`make parity`**: Every scene with a parity port under `tests/oo/scenes/` is bit-exact against its `qa/golden/` frame.
- **`make det`**: Two consecutive builds are byte-identical.

### The Determinism Invariant
> **A change may not break scene parity (`make parity`) or deterministic builds (`make det`) without a written justification in the commit message.**

---

## 3. Capability Discipline & Crash-Resilience Standards

### The Fail-Closed Invariant
- **No silent failure swallowing in production paths**: Shell probes that must distinguish success from failure end in `&& echo <marker>` and test the marker — never `|| true` on a load-bearing command.
- **Fail closed on unknown input**: Unknown CLI verbs, control frames, and scene names produce `ERR` replies and non-zero exits. Control-plane parsing never panics, never blocks unboundedly (bounded `*_timeout` reads only).

### The Jail Discipline
- **`OODA_NO_JAIL` is never set by the build or the shipped units.** The FS jail (`OODA_FS_READDIR` / `OODA_FS_WRITEDIR`) is always on; pages that need host files must work through declared read/write scopes.
- **Environment reads go through `core/environ.oo`**, never raw `getenv` of undeclared keys: the runtime only exposes `OODA_*`/`OO_*` keys, so host values resolve via the launch environment block with shell fallbacks.
- **Atomic writes**: Persistent state (config, artefacts) is written via temp-file + rename, and never clobbers a file the parser cannot read.

### Async & Event-Loop Protection
- **Zero Blocking Accept Without Timeout on Control Paths**: Client-facing reads use bounded timeouts; the reactor never parks forever on one peer.
- **Bounded Frames**: Control frames are size-bounded (4 KiB); overlong input fails closed.
- **Resource cleanup is deterministic**: Socket files are unlinked on clean shutdown so a crashed predecessor never wedges startup (stale paths are also unlinked before bind).

### Pure openOODA Dependencies
- Rely strictly on the `std` capability libraries (`std/fs`, `std/net`) plus the `oodar` host runtime. No shell-out for logic the language can express; shells are for host probing only, with every argument passed through `shell_quote`.

---

## 4. Native systemd Citizenship

`idlescreen` daemons run as first-class systemd user services. They must adhere to native systemd lifecycle and security primitives:

### Socket Layout
- The control plane is a per-user Unix socket at `$XDG_RUNTIME_DIR/<persona>/control.sock`, created mode `0600` by the runtime. No TCP listeners, no other-host reachability.

### Lifecycle
- **Accurate status**: `status` reports the live reactor state; `service` verbs verify backend results and return non-zero when activation fails.
- **Immediate shutdown**: `quit`/`shutdown` frames unwind the reactor, close the listener, and unlink the socket file.

### Standardized Exit Codes (`sysexits.h`)
- On fatal termination, return standard systemd exit codes:
  - `78` (`EX_CONFIG`): Configuration or policy validation failure. Systemd marks the unit failed permanently without crash-looping.
  - `69` (`EX_UNAVAILABLE`): Missing required Wayland compositor or display.
  - `0` (`EX_OK`): Clean shutdown on signal or `quit`.

---

## 5. Desktop Presentation, Power & Sandbox Discipline

`idlescreen` operates within active user desktop sessions. It must respect desktop power, smoothness, and security constraints:

### The "Zero-Wakeup" Idle Invariant
- When user activity is ongoing and no screensaver is active, `idlescreen daemon` must consume near-zero CPU.
- **Zero Polling in Steady State**: All idle monitoring must be event-driven (Wayland `ext-idle-notify-v1` events multiplexed with control-socket wakeups). No periodic `sleep()` loops in background monitoring threads.

### Steady-State Allocation & Frame Budget
- In screensavers running at 60 FPS, the frame deadline is 16.6ms.
- **Bounded Per-Frame Allocation**: Frame buffers are arena-scoped per frame (`frame_arena_open` / `leave` / `close`); steady state holds no growth across frames.

### Power-Adaptive Scaling
- On battery power, savers must conserve laptop energy by either automatically blanking, capping frame rates (e.g. 30 FPS), or disabling expensive multi-pass shaders.

### Honest Diagnostics
- `doctor` reports only measured state: the control endpoint path, the real inhibitor registry answer, the real config path. Unknown states print as unknown — never as healthy. The nominal verdict requires the daemon to be actually running.

---

## 6. Verification Ladder & QA Gate

Before any change is committed or marked complete, every step of the Verification Ladder must pass in order:
1. **Portability**: `make portable` must pass with zero violations.
2. **Page Laws**: `make line-cap file-law academy density vocab shape` must pass with zero violations.
3. **Typecheck**: `make check` (`oodac check` on every `.oo` file) must pass with zero errors.
4. **Test Suite**: `make test` must pass completely.
5. **Scene Parity**: `make parity` must pass — every ported scene bit-exact against its golden.
6. **Determinism**: `make det` must pass — two consecutive builds byte-identical.
7. **Dogfood Proof**: The rebuilt binary must answer `list`, `status`, `config`, `doctor`, and one `preview --frames` run from a clean shell.

---

## 7. Multi-Engine Audit Protocol (Pre-Release)

Advisory tiered audit run before tagging releases, budgeted by token capacity:
1. **`mcode`** (Abundant tokens): Full codebase scan for contract mismatches, packaging specs, and CI gates.  
   `mcode exec --permission full "<prompt>"`
2. **`vibe`** (Moderate tokens): Adversarial diff inspection for subtle regressions, security bypasses, and API validity.  
   `vibe -p "<prompt>" --trust --auto-approve`
3. **`grok`** (Strict quota): High-value referee and final sanity check on critical release diffs.  
   `grok -p "<prompt>" --always-approve`

**Rule:** External audits are strictly advisory; all findings must be verified against source code. External LLMs never gate CI—the §6 Verification Ladder is the sole canonical authority.
