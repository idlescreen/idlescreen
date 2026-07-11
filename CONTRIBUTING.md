# Contributing

Thanks for your interest in Crateria projects.

## Setup

- Rust toolchain from `rust-toolchain.toml` when present
- Run `cargo test` / `cargo clippy` before opening a PR
- For packaging changes, coordinate with [crateria/packages](https://github.com/crateria/packages)

## Pull requests

- Target `main`
- Keep changes focused; include a short rationale in the PR body
- Do not commit secrets, GPG private keys, or unrelated binary artifacts

## Install docs

Prefer the canonical APT keyring path (`/etc/apt/keyrings` + `signed-by`) and DNF `.repo` curl used across Crateria READMEs.
