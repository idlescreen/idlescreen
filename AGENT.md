# AGENT.md — IdleScreen hardening contract

Enforce on every change in this repository.

## Rust and license

- Strict Rust from first principles. Prefer `std` before crates.
- License: **Apache-2.0** only for this project and new code.
- Vetted dependencies only; no `unsafe` without a short safety comment.

## Constraints

- Max **250 lines** per `.rs` file (split at functional boundaries).
- **Zero** `.unwrap()` or `.expect()` in production (non-test) code.
  Tests may use `unwrap`/`expect` only when the failure message is clear.
- Fallible APIs use `Result` / `Option` with explicit error types.

## Testing

- Target about **3:1** test lines to production lines where practical.
- Protocol, parsing, path safety, and package index logic must have
  **proptest** (or equivalent property) tests.
- Prefer deterministic unit tests; integration tests for CLI boundaries.

## Product identity

- GitHub org and product brand: **IdleScreen** (`idlescreen`).
- Historical crate, binary, and Debian package names (`trance`, `trance-*`)
  remain for API and install stability until a coordinated major rename.
- Historical D-Bus well-known names stay for ABI continuity.

## Git edges

- Default branch: **`master`**.
- After each hardening stage barrier: declarative commit message and push to `origin master`.

## Architecture law

- First-principle boundaries: **[docs/BOUNDARIES.md](docs/BOUNDARIES.md)**
  (org canonical copy: idlescreen/.github `docs/BOUNDARIES.md`).
- Stay a Wayland client and plugin host. Do not become a compositor, lock
  screen, or raw DRM/KMS application.
