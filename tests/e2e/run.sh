#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# IdleScreen End-to-End Test Suite Runner (4-Tier Architecture)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=================================================================="
echo "    IdleScreen End-to-End Opaque-Box Test Suite Runner           "
echo "=================================================================="
echo "Workspace: $ROOT_DIR"
echo "Manifest:  $SCRIPT_DIR/Cargo.toml"
echo ""

cd "$ROOT_DIR"

echo "==> Running All 4 Tiers of E2E Tests..."
cargo test --manifest-path "$SCRIPT_DIR/Cargo.toml" --quiet

echo ""
echo "=================================================================="
echo "  E2E Test Execution Summary:                                     "
echo "  ✓ Tier 1: Feature Coverage (R1-R6)                   PASSED     "
echo "  ✓ Tier 2: Boundary & Corner Cases (R1-R6)            PASSED     "
echo "  ✓ Tier 3: Cross-Feature Combinations (Pairwise)      PASSED     "
echo "  ✓ Tier 4: Real-World Scenarios (Workflows)           PASSED     "
echo "=================================================================="
echo "  STATUS: ALL TESTS GREEN (Exit Code 0)                           "
echo "=================================================================="
exit 0
