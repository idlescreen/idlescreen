#!/bin/bash
# removal_stack_check.sh — the product-removal list must cover the whole
# product set.
#
# The list lives in three places (this script's package, the packages
# repo's uninstall.sh, and the packages repo's PRODUCT_STACK_ON_REMOVE
# constant). They are kept in sync by comment, which is not a mechanism:
# a comment does not fail when someone edits one list and not the others.
# That is how the studio suite was left behind by every removal path.
#
# This check pins the idlescreen-side list. The packages-side lists are
# pinned by the packages repo's own contract test.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$ROOT/libexec/remove-product-stack.sh"

fail=0
check() {
    local desc="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        echo "PASS $desc"
    else
        echo "FAIL $desc"
        fail=1
    fi
}

if [ ! -f "$SCRIPT" ]; then
    echo "FAIL removal script not found at $SCRIPT"
    exit 1
fi

# Every package the installer seats must be removable by the same path.
EXPECTED=$(cat <<'EOF'
idle-daemon idle-cli idle-tui idle-savers idle-cosmic
idle-saver-ascii idle-saver-aurora idle-saver-beams
idle-saver-bursts idle-saver-chaos idle-saver-cosmos
idle-saver-glyphs idle-saver-gnats idle-saver-hearth
idle-saver-radar idle-saver-ripple idle-saver-storm
idle-studio render
EOF
)
EXPECTED=$(printf '%s' "$EXPECTED" | tr '\n' ' ')

for pkg in $EXPECTED; do
    check "removal list covers $pkg" grep -qx "$pkg" "$SCRIPT"
done

# …and the list must not name anything outside that set. `render` is an
# IdleScreen package from the studio repo that simply does not start with
# "idle-", so membership is checked against the expected set rather than
# against a name prefix.
LISTED=$(sed -n '/^PKGS="/,/^"$/p' "$SCRIPT" | tr -d '"' | grep -v '^PKGS=$' | grep -v '^$')
for pkg in $LISTED; do
    case " $EXPECTED " in
        *" $pkg "*) ;;
        *) echo "FAIL removal list names an unexpected package: $pkg"; fail=1 ;;
    esac
done

if [ "$fail" -ne 0 ]; then
    echo "removal stack check FAILED"
    exit 1
fi
echo "removal stack check passed"
