# Trance Core Workspace Justfile
# Use "just <recipe>" to execute targets

# Build all binaries in release mode
build:
    cargo build --release

# Run all workspace unit tests
test:
    cargo test --workspace

# Package all core binaries, update local APT pool, and sign metadata
package:
    rustc package.rs -o package_runner
    ./package_runner
    rm -f package_runner

# Check lines of code constraints (between 25 and 250 lines)
check-limits:
    rustc trance-runner/check_limits.rs -o check_limits_runner
    cd trance-runner && ../check_limits_runner
    rm -f check_limits_runner
