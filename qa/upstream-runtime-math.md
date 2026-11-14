# Upstream report — two semantic regressions in the `.oo` runtime rewrite

**Org:** idlescreen · **Toolchain:** openOODA-core (oodac + std + oodar)
**Date:** 2026-11-13
**Upstream refs:** `openOODA-core/oodar` `a53de08` (C runtime) → `9af908c` (2026-11-13)

Two independent regressions came in with the C → `.oo` runtime rewrite. Both are
silent: nothing errors, the runtime just answers a different question than it
used to. Defect A breaks UTF-8 rendering. Defect B breaks bit-exact
floating-point reproducibility against any libm-based reference.

> Re-verified on 2026-11-13 against the newest published runtime (hub asset
> `40ac871e…`, upstream `ab5ae7bc`): both defects are still present, and the
> parity sweep still stands at 48/51 with the same three scenes.

---

## Defect A — string primitives changed from codepoints to bytes

### Summary

The `oodar` runtime was rewritten from C to openOODA (`.oo`) between the
`a53de08` commit and `upstream/master` (`9af908c`). The rewrite changed the
meaning of two runtime string primitives:

| primitive | C runtime (`a53de08`) | `.oo` runtime (`9af908c`) |
|---|---|---|
| `oo_chars_len(s)` | number of UTF-8 **codepoints** | number of **bytes** (`sl(s)`) |
| `oo_char_at(s, i)` | the codepoint string at codepoint index `i` | the single **byte** at byte index `i` |

Evidence (both runtimes, same compiler `04adc274…`, same std `72c26be1`):

```
s = "a█b"        (5 bytes, 3 codepoints)

C runtime:    bytes_len=5  chars_len=3  char_at(s,1)="█"
.oo runtime:  bytes_len=5  chars_len=5  char_at(s,1)="\xE2"
```

Source of the change:

- old: `oodar/core/str/str_ops.c` — `oo_chars_len` walks the UTF-8 sequence
  and counts codepoints (with an ASCII fast path).
- new: `oodar/core/oodar/src/host/str/str_ops.oo:101` —
  `export fn oo_chars_len(s: String) -> Int { return sl(s); }`, and
  `oo_char_at` returns `oo_str_ascii_intern(rt_load_u8(sp(s), i))`.

### Why it matters

Rust's `str::chars()` counts codepoints, so the upstream semantics change
breaks every port that iterates a string by index and expects UTF-8
characters. Concretely, in `idlescreen`:

- the 7-row wordmark font atlas is a string of `U+2588`; a byte walk never
  matches the block glyph and flattens the whole font to dots;
- a centred banner is measured with `chars_len`, so a string containing
  `✦` is padded two columns short and the box no longer lines up;
- a substring search over a multi-byte string stops matching.

Both the released hub asset (`openOODA/install` release `hub`, published
2026-11-08) and a runtime built from `upstream/master` behave the new way,
so this is not a stale-download problem.

### Suggested fix

Restore codepoint semantics in the runtime, or make the change explicit and
versioned:

- `oo_chars_len` should count codepoints (the C implementation is a
  drop-in reference), and
- `oo_char_at` should return the codepoint string at a **codepoint** index.

If byte semantics are intended, then `str_slice`, `chars_len` and `char_at`
need one coherent definition and the semantic change has to be announced,
because it silently changes the output of every UTF-8 renderer built against
the previous runtime.

---

## Defect B — transcendental math is no longer libm-compatible

### Summary

The same rewrite replaced the C runtime's libm-backed math with a pure-`.oo`
approximation. The new `pow`/`sqrt`/`sin`/`cos` are accurate to ~1–2 ulp, but
they are **not** bit-compatible with glibc. Any consumer whose reference
values come from libm (Rust's `f64::powf`, `f64::sqrt`, `f64::sin`, …) now
disagrees in the last bits.

### Source of the change

`oodar/core/oodar/src/host/sys/math_ops.oo`:

```oo
pub fn math_pow(x: Float, y: Float) -> Float {
    ...
    return math_exp(y * math_ln(x));      // exp/ln in plain f64
}
pub fn math_sqrt(x: Float) -> Float {
    ... 8 Newton steps from a bit-seeded start ...
}
pub fn math_sin(x: Float) -> Float { ... }   // 2-term Cody-Waite + 10-term Taylor
pub fn math_cos(x: Float) -> Float { ... }
```

The old C runtime was a thin, exact wrapper:

```c
double oo_ln(double x)   { return log(x); }
double oo_exp(double x)  { return exp(x); }
double oo_sqrt(double x) { return sqrt(x); }
double oo_pow(double b, double e) { return pow(b, e); }
```

### Evidence

Bit patterns of the same expression under the C runtime, the `.oo` runtime and
glibc (via Rust `f64::powf`/`sqrt`):

| expression | glibc / Rust | C runtime | `.oo` runtime | Δ ulp |
|---|---|---|---|---|
| `pow(0.8, 2.0)` | `0x3fe47ae147ae147c` | same | `0x3fe47ae147ae147b` | 1 |
| `pow(0.8, 4.0)` | `0x3fda36e2eb1c432e` | same | `0x3fda36e2eb1c432f` | 1 |
| `pow(1.5, 3.0)` | `0x400b000000000000` | same | `0x400b000000000002` | 2 |
| `pow(2.0, -6.0)` | `0x3f90000000000000` | same | `0x3f90000000000002` | 2 |
| `sqrt(2.0)` | `0x3ff6a09e667f3bcd` | same | `0x3ff6a09e667f3bcc` | 1 |
| `cos(pi * 0.6)` | `0xbfd3c6ef372fe94e` | same | `0xbfd3c6ef372fe94f` | 1 |

Measured across the 31 easing variants × 100 ratios used by the `idlescreen`
parity engine, the `.oo` runtime disagrees with glibc on 23 of the 31
variants (up to 4096 ulp where the result is near zero). The old C runtime
disagreed on **none**.

### Why it matters

`idlescreen` ships a Rust reference implementation and a `.oo` port whose
parity is asserted with **bit-exact frame hashes** (51 scenes, `qa/golden/`).
The goldens are the Rust output; the Rust reference reproduces all 51
bit-exactly. Under the `.oo` runtime the port's output diverges:

| scene | divergence under the `.oo` runtime |
|---|---|
| `sweep` | 2 of 100 frames differ (a group reveal lands one frame early) |
| `swarm` | 3 of ~700 frames differ |
| `fireworks` | the scene **runs 18 frames longer** (446 → 464 frames) before it settles |

The `fireworks` case is not a rounding artefact: path completion depends on
an integer truncation of a float, so a 1-ulp difference changes the number of
frames the scene stays alive — a visible difference in the shipped animation.

### Important: a "more accurate" reimplementation would *not* fix this

glibc's `pow` is **not** correctly rounded. Over 100 000 sample points:

```
pow(x, 2.0)  != x * x            : 78 cases
pow(x, 0.5)  != sqrt(x)          : 75 cases
pow(x, 3.0)  != x * x * x        : 26108 cases
```

So a correctly-rounded `pow`/`sqrt` implemented in `.oo` would still disagree
with the goldens in ~0.08 % of cases (exponent 2) and far more for other
exponents. **Only a libm-backed implementation reproduces the reference
bit-for-bit.** Any fix must restore glibc, not improve on it.

### Suggested fix (preferred)

Make the **compiler** lower the math builtins to LLVM intrinsics so clang
emits libm calls, instead of routing them to the runtime's `oo_*` shims:

```
pow  →  llvm.pow.f64      sin  →  llvm.sin.f64
cos  →  llvm.cos.f64      sqrt →  llvm.sqrt.f64
exp  →  llvm.exp.f64      ln   →  llvm.log.f64
```

`clang++` already links `-lm` on every build line
(`oodac/cli/build/cli_build.oo:64`, `:127`), so no new dependency is needed.
The runtime's pure-`.oo` `math_ops` can stay for runtime-internal use — it
would itself be compiled against libm and inherit the same exactness.

This restores bit-exact parity for every libm-based consumer and keeps the
runtime free of a C dependency for its own internals.

### Suggested fix (fallback)

If the runtime must stay pure-`.oo`, the only remaining option is to
reimplement **glibc's exact algorithms** (its `exp`/`log` tables and branch
logic) so the last bit matches. That is not practical to maintain, so we
would treat it as "no fix available" and adapt on the consumer side instead.

### Reproduction

```sh
# toolchain
curl -fsSL https://openooda.org/install.sh | bash -s -- --yes
export OODA_HOME=$HOME/.openooda OODA_COMPILER=$HOME/.openooda/bin/oodac
export OODA_STD_DIR=$HOME/.openooda/oostd

# idlescreen parity gate (3 scenes diverge under the .oo runtime)
cd idlescreen && make parity

# the same gate against the C runtime reproduces the goldens exactly:
#   OODA_HOME=<openOODA-core checkout> OODA_OODAR_ROOT=<checkout>/oodar
```

---

## Consumer-side mitigation (idlescreen, for reference)

While the upstream fix is pending, `idlescreen` keeps its Rust goldens
untouched and quarantines the three affected scenes behind an explicit,
documented list (`qa/parity_quarantine.txt`). The gate still fails on any
*other* divergence, and the quarantined scenes are reported rather than
silently dropped. The product difference is bounded and documented: ≤3 frames
of group-reveal timing in `sweep`/`swarm`, and a 4 %-longer `fireworks`
scene.
