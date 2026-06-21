#!/bin/bash
# trance - local packaging script
set -e

SC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGES_DIR="$(cd "$SC_DIR/../apt" && pwd)"

echo "=========================================="
echo "Building All Trance Packages..."
echo "=========================================="

export PATH="$HOME/.cargo/bin:$PATH"

echo "Compiling release binaries..."
cargo build --release

CRATES=(
    "trance-daemon"
    "trance-cli"
    "trance-plugins-all"
    "trance-applet"
)

for crate in "${CRATES[@]}"; do
    echo "------------------------------------------"
    echo "Packaging: $crate"
    echo "------------------------------------------"

    echo "Building Debian package for $crate..."
    cargo deb --no-build -p "$crate"

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

echo "=========================================="
echo "Updating packages index..."
echo "=========================================="
cd "$PACKAGES_DIR"
./update.sh
cd "$SC_DIR"

echo "=========================================="
echo "Trance build and package sync complete!"
echo "=========================================="

echo ""
echo "Do you want to commit and push these package updates to GitHub? (y/n)"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    VERSION=$(grep -m 1 '^version =' trance-daemon/Cargo.toml | cut -d '"' -f 2)
    cd "$PACKAGES_DIR"
    git add .
    git commit -m "Release trance v$VERSION"
    git push origin main
    cd "$SC_DIR"
    echo "Push complete. You can now run 'sudo apt update && sudo apt install trance' on client machines."
fi