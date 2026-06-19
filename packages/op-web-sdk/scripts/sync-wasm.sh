#!/usr/bin/env bash
# Build the Plan-1 wasm bundle and copy its output into this package's wasm/.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
bash "${ROOT}/crates/op-web-sdk/tools/build-wasm.sh"
DST="$(cd "$(dirname "$0")/.." && pwd)/wasm"
mkdir -p "${DST}"
cp -R "${ROOT}/crates/op-web-sdk/pkg/." "${DST}/"
echo "synced wasm bundle → ${DST}"
# Vendor the ts-rs generated types into the package so it is self-contained.
# Post-process: remove the dangling serde_json import and inline JsonValue so
# the vendored file compiles without the missing ./serde_json/JsonValue module.
PKGROOT="$(cd "$(dirname "$0")/.." && pwd)"
cp "${ROOT}/crates/op-web-sdk/bindings/ops.ts" "${PKGROOT}/src/ops-types.ts"
perl -i -0pe '
  # Remove the ts-rs generated import for the missing serde_json helper.
  s/import type \{ JsonValue \} from "\.\/serde_json\/JsonValue";\n//g;
  # Inject the GENERATED header comment and the inline JsonValue definition
  # right after the first comment block (the ts-rs file header line), but only
  # when the inline block is not already present (idempotent).
  unless (/\/\/ JsonValue inlined/) {
    s|(// This file was generated.*?\n)|\1\n// JsonValue inlined from the ts-rs serde_json helper (the original imports from\n// \.\/serde_json\/JsonValue which is not part of this vendored package).\ntype JsonValue = null \| boolean \| number \| string \| JsonValue[] \| \{ \[key: string\]: JsonValue \};\n|s;
  }
' "${PKGROOT}/src/ops-types.ts"
echo "synced ops-types → ${PKGROOT}/src/ops-types.ts"
