#!/bin/sh
# Vendor escape for `libcosmic` (Sprint 05 follow-up).
#
# When upstream `libcosmic` history is force-pushed and the pinned commit
# (rev = "ef162b8e") is no longer reachable, this script switches the
# dependency to a local vendored copy under `vendor/libcosmic/`.
#
# This script does NOT clone from upstream (the SHA is already gone by
# the time you need the escape). The vendored copy must exist on disk
# before invoking this script. Set it up one of these ways:
#
#   - `git clone https://github.com/pop-os/libcosmic.git vendor/libcosmic`
#     followed by `git -C vendor/libcosmic checkout ef162b8e`
#     — only works if the SHA is still on the upstream server.
#   - Restore from your own CI artifact cache: many CI setups vendor
#     libcosmic on every release. Copy from the cache to vendor/libcosmic.
#   - Vendor from a known-good mirror: a private fork of libcosmic at
#     the pinned SHA.
#
# Usage:
#   ./scripts/vendor_libcosmic.sh            # switch to path dep (idempotent)
#   ./scripts/vendor_libcosmic.sh restore     # restore the git-pin Cargo.toml
#
# Idempotent: re-running is safe.

set -eu

VENDOR_DIR="vendor/libcosmic"
CARGO_TOML="Cargo.toml"
BACKUP=".libcosmic-pin.bak"

# Take a one-time backup of the original [dependencies.libcosmic] block.
if [ ! -f "$BACKUP" ]; then
    if grep -q 'git = "https://github.com/pop-os/libcosmic.git"' "$CARGO_TOML"; then
        sed -n '/\[dependencies.libcosmic\]/,/^\[/p' "$CARGO_TOML" > "$BACKUP" 2>/dev/null || true
    fi
fi

case "${1:-vendor}" in
    vendor)
        if [ ! -d "$VENDOR_DIR" ]; then
            cat <<EOF >&2
vendor: $VENDOR_DIR does not exist.

To create a vendored copy, do ONE of the following:
  1. If upstream still has the SHA:
       git clone https://github.com/pop-os/libcosmic.git $VENDOR_DIR
       ( cd $VENDOR_DIR && git fetch origin ef162b8e && git checkout ef162b8e )
  2. From a CI cache: copy libcosmic @ ef162b8e to $VENDOR_DIR
  3. From a mirror: clone your private fork at the pinned SHA

After the directory exists, re-run this script.
EOF
            exit 1
        fi
        if [ ! -f "$BACKUP" ]; then
            echo "vendor: cannot find pinned fragment in $CARGO_TOML; may already be path-pinned" >&2
            echo "        run with 'restore' first to revert" >&2
            exit 1
        fi
        echo "Switching $CARGO_TOML to path dep on $VENDOR_DIR"
        awk '
            /^\[dependencies.libcosmic\]/ { in_block = 1
                print "[dependencies.libcosmic]"
                print "path = \"vendor/libcosmic\""
                print "# Path-pinned; original git-pin restored from .libcosmic-pin.bak"
                next
            }
            in_block && /^\[/ { in_block = 0 }
            !in_block { print }
        ' "$CARGO_TOML" > "$CARGO_TOML.tmp" && mv "$CARGO_TOML.tmp" "$CARGO_TOML"
        echo "Done. Run \`cargo build\` to verify."
        echo "    Restore with: $0 restore"
        ;;
    restore)
        if [ ! -f "$BACKUP" ]; then
            echo "restore: no backup at $BACKUP (was the original already restored?)" >&2
            exit 1
        fi
        echo "Restoring original git-pin fragment"
        awk -v backup="$BACKUP" '
            /^\[dependencies.libcosmic\]/ {
                in_block = 1
                while ((getline line < backup) > 0) print line
                close(backup)
                in_block = 0
                next
            }
            in_block && /^\[/ { in_block = 0 }
            !in_block { print }
        ' "$CARGO_TOML" > "$CARGO_TOML.tmp" && mv "$CARGO_TOML.tmp" "$CARGO_TOML"
        rm -f "$BACKUP"
        echo "Done. Original pin restored."
        ;;
    *)
        echo "usage: $0 [vendor|restore]" >&2
        exit 2
        ;;
esac