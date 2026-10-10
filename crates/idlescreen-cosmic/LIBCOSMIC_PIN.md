# `libcosmic` pin policy — `idle-cosmic`

## Current pin

```toml
[dependencies.libcosmic]
git = "https://github.com/pop-os/libcosmic.git"
rev = "ef162b8e"
```

We pin to a specific commit rather than a tag or branch. Rationale:

1. **`libcosmic` does not cut stable releases** in the cadence the applet
   requires. Tags exist (`v0.12` at `9c62f19e` is the latest as of this
   writing) but they track `main`-line snapshots and have moved several
   breaking changes between them.
2. **The pinned commit** (`ef162b8e`) is a known-good state we built
   against. It builds clean against our `iced` / `tokio` / `wayland`
   feature set; the applet's widgets (`button`, `icon`, `text`) all
   compile without warnings.
3. **Reproducible builds**: a commit hash pins *exactly* what was tested.
   A tag can be force-pushed or re-tagged; a commit SHA cannot.

## Risks

- **Upstream force-push**: the commit hash may vanish from the
  `libcosmic` Git history if the maintainer force-pushes `main`. We've
  not seen that happen, but it's possible. Mitigation: vendor a copy
  under `vendor/libcosmic/` if upstream history is rewritten (see
  "Vendor escape" below).
- **Feature drift**: when a new COSMIC release lands, our applet will
  be missing new widgets. Cosmetic only — the runtime contract (D-Bus
  control + preset saver) is unchanged.

## When to bump

Bump the `rev` when **all** of the following are true:

1. The applet builds clean against the new commit on:
   - x86_64-unknown-linux-gnu (Fedora, Arch, Debian)
   - aarch64-unknown-linux-gnu (Asahi, Fedora ARM, RPi)
2. The CI workflow (`.github/workflows/ci.yml`) passes with all features
   enabled (`applet, applet-token, dbus-config, multi-window, tokio,
   wayland, winit`).
3. We've tested the applet interactively against a COSMIC session at
   the matching release.

The recommended update procedure:

```sh
# Fetch upstream main, find the tip commit you want to pin to.
git ls-remote https://github.com/pop-os/libcosmic.git refs/heads/main

# Edit Cargo.toml: replace `rev = "<old>"` with the new SHA.
$EDITOR Cargo.toml

# Build clean and bump the idle-cosmic version in lockstep.
cargo update
cargo build --release
# Update package version to mark the new build baseline.
$EDITOR Cargo.toml  # bump version

# Open a PR. CI gates the bump.
git commit -am "chore: bump libcosmic pin to <new-sha>"
```

## Vendor escape

If upstream `libcosmic` history is rewritten and the pinned commit
disappears, we can vendor it locally:

```sh
git clone --depth=1 --branch=<sha> https://github.com/pop-os/libcosmic.git vendor/libcosmic
```

Then change the dep to:

```toml
[dependencies.libcosmic]
path = "vendor/libcosmic"
features = [...]
```

Cargo's `[patch.crates-io]` and `[patch."ssh://..."]` sections also work
if we want to keep the git URL but redirect to a vendored fork.

## See also

- `Cargo.toml` — current pin
- `.github/workflows/ci.yml` — CI matrix that gates bumps
## Advisory status (2026-09-11)

`cargo audit` reports two `unsound` transitive deps, both pinned *by
libcosmic's own tree* — bumping `rev` does not clear them (verified on
`a401af8`):

- `lru 0.16.4` — RUSTSEC-2026-0253 (UAF in `LruCache::pop()`, needs a
  panic inside eviction to trigger). Reachable via `cryoglyph` text
  shaping. Exposure: low — the applet renders its own static
  icon/tooltip; no attacker-controlled text shapes.
- `memmap2 0.8.0` — RUSTSEC-2026-0186 (unchecked pointer offset) via
  `xkbcommon 0.7`. Exposure: low — consumes system keymaps, not a
  hostile input boundary.

A straight `rev` bump to `a401af8` also breaks `cosmic::app::Action::
Surface` — the applet needs API porting to move pins. Revisit when
upstream (iced/xkbcommon/cryoglyph) releases the fixes; the audit
finding stays open but is not locally actionable.
