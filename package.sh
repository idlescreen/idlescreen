#!/bin/bash
# trance - local packaging script
set -e

# Get the directory of the script
SC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGES_DIR="$(cd "$SC_DIR/../apt" && pwd)"

echo "=========================================="
echo "Building All Trance Packages..."
echo "=========================================="

# Ensure ~/.cargo/bin is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Compile everything in release mode (once for all crates)
echo "Compiling release binaries..."
cargo build --release

# Packages to build and copy
CRATES=(
    "trance-tui"
    "trance-daemon"
    "trance-applet"
)

# Loop through each crate to package it
for crate in "${CRATES[@]}"; do
    echo "------------------------------------------"
    echo "Packaging: $crate"
    echo "------------------------------------------"

    # 1. Build Debian (.deb) package
    echo "Building Debian package for $crate..."
    cargo deb --no-build -p "$crate"
    
    # Map workspace crate names to output package names (trance-daemon outputs as trance)
    pkg_name="$crate"
    if [ "$crate" = "trance-daemon" ]; then
        pkg_name="trance"
    fi

    DEB_FILE=$(ls target/debian/"$pkg_name"_*.deb 2>/dev/null | head -n 1 || true)
    if [ -n "$DEB_FILE" ]; then
        echo "Built: $DEB_FILE"
        cp "$DEB_FILE" "$PACKAGES_DIR/pool/main/"
        echo "Copied to apt repository pool/main/"
    else
        echo "Warning: Debian package not found for $crate (searched for package: $pkg_name)."
    fi
done

# 2. Trigger Packages Repo Update
echo "=========================================="
echo "Updating packages index..."
echo "=========================================="
cd "$PACKAGES_DIR"
./update.sh
cd "$SC_DIR"

echo "=========================================="
echo "Trance build and package sync complete!"
echo "=========================================="

# 3. Optional commit and push
echo ""
echo "Do you want to commit and push these package updates to GitHub? (y/n)"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    VERSION=$(grep -m 1 '^version =' trance-tui/Cargo.toml | cut -d '"' -f 2)
    cd "$PACKAGES_DIR"
    git add .
    git commit -m "Release trance v$VERSION"
    git push origin main
    cd "$SC_DIR"
    echo "Push complete. You can now run 'sudo apt update && sudo apt install trance' on client machines."
fi
