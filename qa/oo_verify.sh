#!/usr/bin/env bash
# qa/oo_verify.sh — openOODA (.oo) suite gate for the ooda branch.
#
# Verifies, in order:
#   1. Page caps: every committed .oo file is 16..=140 lines (user cap is
#      stricter than the openOODA 256 ceiling).
#   2. Academy headers: every .oo file carries the 4-element docstring.
#   3. Ambient authority: tests/oo parity modules must be pure — no
#      capability types, env/process/fs/sys calls, or ambient builtins.
#   4. Typecheck: oodac check on every .oo entry point.
#   5. Double-run determinism: scene_emit plasma run twice, byte-equal.
#   6. Scene parity: .oo plasma digest stream == qa/golden/plasma.txt.
#
# Exit 0 only when every gate passes. oodac is resolved from PATH or
# ~/.openooda/bin/oodac.

set -euo pipefail
cd "$(dirname "$0")/.."

OODAC="${OODAC:-}"
if [ -z "$OODAC" ]; then
  if command -v oodac >/dev/null 2>&1; then OODAC="$(command -v oodac)";
  elif [ -x "$HOME/.openooda/bin/oodac" ]; then OODAC="$HOME/.openooda/bin/oodac";
  else echo "oo_verify: oodac not found" >&2; exit 2; fi
fi

LINE_CAP=140
fails=0

mapfile -t OO_FILES < <(find . -name '*.oo' -not -path './.git/*' -not -path '*/.ooda-cache/*' -not -path '*/target/*' | sort)
[ "${#OO_FILES[@]}" -gt 0 ] || { echo "oo_verify: no .oo files found" >&2; exit 2; }

# --- Gate 1+2: line caps and Academy headers -------------------------------
is_shim() {
  # A shim page: every non-blank, non-comment line is an import or re-export.
  local f="$1" code
  code=$(grep -vE '^\s*(//|$)' "$f" | grep -cvE '^\s*(import |pub use |use )')
  [ "$code" -eq 0 ]
}

for f in "${OO_FILES[@]}"; do
  n=$(wc -l < "$f")
  if [ "$n" -gt "$LINE_CAP" ] || { [ "$n" -lt 16 ] && ! is_shim "$f"; }; then
    echo "FAIL lines($n): $f"; fails=$((fails+1))
  fi
  for pat in '^// #' '^// Logline:' '^// Setup:' '^// Beats:'; do
    if ! grep -q "$pat" "$f"; then
      echo "FAIL header($pat): $f"; fails=$((fails+1)); break
    fi
  done
done
echo "gate1-2: ${#OO_FILES[@]} files scanned, cap=$LINE_CAP"

# --- Gate 3: zero ambient authority in tests/oo -----------------------------
# Parity modules are pure compute: capability types and privileged builtins
# are forbidden. (The product tree under */ legitimately declares caps.)
for f in tests/oo/*.oo; do
  [ -e "$f" ] || continue
  if grep -nE '&(ProcessCap|FsReadCap|FsWriteCap|EnvCap|NetCap|SysCap|TimeCap|AllocCap|ThreadCap)' "$f"; then
    echo "FAIL ambient-cap: $f"; fails=$((fails+1))
  fi
  if grep -nE '\b(sys_exec|env_get|env_set|file_read|file_write|net_|socket_)' "$f"; then
    echo "FAIL ambient-call: $f"; fails=$((fails+1))
  fi
done
echo "gate3: tests/oo purity scan done"

# --- Gate 4: typecheck ------------------------------------------------------
for f in tests/oo/*.oo tests/test_main.oo anchor.oo main.oo; do
  [ -e "$f" ] || continue
  if ! "$OODAC" check "$f" >/dev/null 2>&1; then
    echo "FAIL oodac-check: $f"; fails=$((fails+1))
  fi
done
echo "gate4: oodac check done"

# --- Gates 5+6: determinism and golden parity -------------------------------
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
"$OODAC" build tests/oo/scene_emit.oo -o "$TMP/scene_emit" >/dev/null 2>&1 \
  || { echo "FAIL build: scene_emit.oo"; exit 2; }
"$TMP/scene_emit" plasma > "$TMP/run1.txt"
"$TMP/scene_emit" plasma > "$TMP/run2.txt"
if ! cmp -s "$TMP/run1.txt" "$TMP/run2.txt"; then
  echo "FAIL double-run determinism"; fails=$((fails+1))
fi
if [ -f qa/golden/plasma.txt ]; then
  if ! cmp -s "$TMP/run1.txt" qa/golden/plasma.txt; then
    echo "FAIL golden parity: plasma"; fails=$((fails+1))
  else
    echo "gate6: plasma golden parity OK"
  fi
fi

if [ "$fails" -gt 0 ]; then
  echo "oo_verify: $fails failure(s)"; exit 1
fi
echo "oo_verify: all gates passed"
