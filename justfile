default: verify

# Build a release binary
build:
    cargo build --release --workspace

# Build a release binary (with hard-coded debug info for crash reports)
build-debug:
    cargo build --release --workspace --profile=release-with-debug

# Run the full test suite
test:
    cargo test --workspace --all-features

# Run doc tests
test-doc:
    cargo test --workspace --doc

# Format check
fmt-check:
    cargo fmt --all -- --check

# Format (apply changes)
fmt:
    cargo fmt --all

# Clippy (deny warnings)
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Documentation build
doc:
    cargo doc --workspace --no-deps --open

# Security audit
audit:
    cargo audit

# License + advisory check
deny:
    cargo deny check

# Find unused dependencies
udeps:
    cargo +nightly udeps --workspace --all-features

# Code coverage (HTML report)
coverage:
    cargo llvm-cov --workspace --all-features --html

# Build distribution packages (deb, rpm)
package:
    ./package.rs

# Enforce the 25-250 line cap (see trance-runner/LINE_LIMITS.md)
check-limits:
    cd trance-runner && cargo run --bin check_limits

# Verify formatting + lint + tests all pass
verify: fmt-check clippy test test-doc
    @echo "All checks passed."

# Quick CI mirror: lint + test only
ci: fmt-check clippy test
    @echo "CI checks passed."