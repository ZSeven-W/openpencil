#!/usr/bin/env bash
# Validate OpenPencil's macOS local-network privacy metadata.
#
# With plist arguments, this checks the exact files. With no arguments, it
# checks the cargo-bundle extension metadata and all repository packaging /
# release integration points.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=macos-local-network-plist.sh
source "$ROOT/tools/macos-local-network-plist.sh"

fail() {
  printf 'macos-bundle-plist: error: %s\n' "$*" >&2
  exit 1
}

require_regex() {
  local file=$1
  local pattern=$2
  local reason=$3
  grep -Eq -- "$pattern" "$file" ||
    fail "$file: $reason"
}

check_plists() {
  OPENPENCIL_EXPECTED_LOCAL_NETWORK_DESCRIPTION="$OPENPENCIL_LOCAL_NETWORK_USAGE_DESCRIPTION" \
    OPENPENCIL_EXPECTED_BONJOUR_SERVICE="$OPENPENCIL_BONJOUR_SERVICE" \
    python3 - "$@" <<'PY'
import os
import plistlib
import sys
from pathlib import Path

expected_description = os.environ["OPENPENCIL_EXPECTED_LOCAL_NETWORK_DESCRIPTION"]
expected_service = os.environ["OPENPENCIL_EXPECTED_BONJOUR_SERVICE"]
failed = False

for raw_path in sys.argv[1:]:
    path = Path(raw_path)
    try:
        with path.open("rb") as handle:
            plist = plistlib.load(handle)
    except (OSError, plistlib.InvalidFileException) as error:
        print(f"macos-bundle-plist: error: {path}: invalid plist: {error}", file=sys.stderr)
        failed = True
        continue

    description = plist.get("NSLocalNetworkUsageDescription")
    if description != expected_description:
        print(
            "macos-bundle-plist: error: "
            f"{path}: NSLocalNetworkUsageDescription must equal "
            f"{expected_description!r}, got {description!r}",
            file=sys.stderr,
        )
        failed = True

    services = plist.get("NSBonjourServices")
    if services != [expected_service]:
        print(
            "macos-bundle-plist: error: "
            f"{path}: NSBonjourServices must equal [{expected_service!r}], "
            f"got {services!r}",
            file=sys.stderr,
        )
        failed = True

if failed:
    raise SystemExit(1)
PY
}

if (($# > 0)); then
  check_plists "$@"
  printf 'macos-bundle-plist: PASS (%s plist file(s))\n' "$#"
  exit 0
fi

check_plists "$ROOT/crates/op-host-desktop/Info.plist"

for bundle_script in scripts/bundle-macos.sh tools/bundle-macos.sh; do
  require_regex \
    "$ROOT/$bundle_script" \
    '^[[:space:]]*(source|[.])[[:space:]]+.*macos-local-network-plist[.]sh' \
    'must source the canonical local-network metadata'
  require_regex \
    "$ROOT/$bundle_script" \
    '^[[:space:]]*openpencil_apply_macos_local_network_plist[[:space:]]+"\$PLIST"[[:space:]]*$' \
    'must patch the final bundle Info.plist through the canonical helper'
  require_regex \
    "$ROOT/$bundle_script" \
    '^[[:space:]]*bash[[:space:]]+".*check-macos-bundle-plist[.]sh"[[:space:]]+"\$PLIST"[[:space:]]*$' \
    'must validate the final bundle Info.plist'
done

release_workflow="$ROOT/.github/workflows/rust-release.yml"
require_regex \
  "$release_workflow" \
  '^[[:space:]]*bash[[:space:]]+scripts/bundle-macos[.]sh[[:space:]]*$' \
  'release workflow must use the production macOS bundle script'
require_regex \
  "$release_workflow" \
  '^[[:space:]]*bash[[:space:]]+tools/check-macos-bundle-plist[.]sh[[:space:]]+\\[[:space:]]*$' \
  'release workflow must validate the assembled app Info.plist before notarization'
require_regex \
  "$release_workflow" \
  '^[[:space:]]*"target/\$\{\{ matrix[.]target \}\}/release/bundle/osx/OpenPencil[.]app/Contents/Info[.]plist"[[:space:]]*$' \
  'release workflow must validate the exact app path it notarizes'

printf 'macos-bundle-plist: PASS (source metadata and release integration)\n'
