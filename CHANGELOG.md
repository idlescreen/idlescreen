# Changelog

All notable changes to `trance` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased] - 2026-07-23

### Changed
- **Audit Wave**: Org-wide consolidation of `shared-assets` usage.
  The shared crate (v3.0.34..v3.0.37) now hosts the workspace
  inheritance, the `rate_limit::RateLimiter`, `session_id::generate_session_id`,
  `cookie_auth::{build_cookie, cookie_should_be_secure}`, the shared
  `Login` component, `app_error::AppError`, and
  `auth::origin_check::{origin_matches, forbidden_response, ...}`.
  The web apps' duplicated auth helpers, config, types, security
  headers, CSS, and `bin/sh/tui.rs` shells have been removed in
  favour of the shared implementations.
- **Pre-wave**: Removed the per-app interactive TUI admin console
  in favour of the existing CUI subcommands.
- **Per-app refactor**: file size cap enforcement (≤ 250 LoC/.rs)
  applied where the audit flagged oversize files.

