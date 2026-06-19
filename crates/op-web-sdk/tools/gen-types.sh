#!/usr/bin/env bash
# gen-types.sh — emit TypeScript type definitions for PenDocument and related
# types by running jian-ops-schema's built-in export_ts binary.
#
# Output: crates/op-web-sdk/bindings/ops.ts  (inside our crate, NOT the submodule)
#
# The jian-ops-schema export_ts binary writes into its own crate's bindings/
# directory (vendor/jian/crates/jian-ops-schema/bindings/) via a compile-time
# CARGO_MANIFEST_DIR path — it cannot be redirected by TS_RS_EXPORT_DIR at
# runtime.  To keep vendor/jian clean we:
#   1. Run the binary (writes into vendor submodule).
#   2. Copy the generated file into crates/op-web-sdk/bindings/.
#   3. Restore vendor/jian to the pre-run state.
# After this script exits, `git -C vendor/jian status --short` must show no
# changes.
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
JIAN_ROOT="${WORKSPACE_ROOT}/vendor/jian"
JIAN_BINDINGS="${JIAN_ROOT}/crates/jian-ops-schema/bindings"
SDK_BINDINGS="${WORKSPACE_ROOT}/crates/op-web-sdk/bindings"

echo "Generating TypeScript bindings from jian-ops-schema (workspace: ${JIAN_ROOT})..."
# jian is excluded from the main workspace — run cargo inside its own workspace.
cargo run --manifest-path "${JIAN_ROOT}/Cargo.toml" \
    -p jian-ops-schema \
    --features export-ts \
    --bin export_ts 2>&1

echo "Copying generated files to ${SDK_BINDINGS}..."
mkdir -p "${SDK_BINDINGS}"
# Copy all .ts files emitted by ts-rs into our crate's bindings directory.
cp "${JIAN_BINDINGS}"/*.ts "${SDK_BINDINGS}/"

echo "Restoring vendor/jian to pre-run state to keep the submodule clean..."
git -C "${JIAN_ROOT}" checkout -- .

echo "Done. TypeScript bindings available at ${SDK_BINDINGS}/"
echo "Verifying vendor/jian is clean..."
if git -C "${JIAN_ROOT}" status --short | grep -q .; then
    echo "ERROR: vendor/jian still has uncommitted changes after restore!" >&2
    git -C "${JIAN_ROOT}" status --short >&2
    exit 1
fi
echo "vendor/jian is clean — OK."
