#!/usr/bin/env bash
# tools/fetch-skia-artifact.sh — Variant A: GitHub release asset
#
# Fetches the Skia WASM static library selected by P0 §3.1 Artifact
# Distribution Strategy (release asset hosting; rationale: <1 MB gzip,
# free hosting, no extra git-lfs setup).
#
# The actual release asset is published in Phase E once the production
# build pipeline lands; for Step 1b kickoff (Phase A onward) the dev
# loop builds Skia from source via skia-bindings, so this script's
# best-effort fetch is allowed to no-op when the release tag is absent.
#
# Exit semantics:
#   0  vendor/skia-wasm/libskia-current.a is present and (when
#      EXPECTED_SHA256 is set) matches the pinned hash; OR the artifact
#      is absent and Phase A from-source build is the active path.
#   1  artifact present but SHA mismatched, OR network/git-lfs failure
#      after a download was attempted.
#
# Phase E will populate EXPECTED_SHA256 + RELEASE_TAG + RELEASE_FILE
# constants below; until then the script logs and returns 0 so dev
# builds keep working without a published release.

set -euo pipefail

DEST_DIR="vendor/skia-wasm"
DEST="${DEST_DIR}/libskia-current.a"

# Phase E will overwrite the next three constants with the published
# release asset metadata. Empty string = "no release yet, fall through
# to from-source build".
RELEASE_TAG=""
RELEASE_FILE=""
EXPECTED_SHA256=""

mkdir -p "$DEST_DIR"

if [ -f "$DEST" ]; then
  if [ -n "$EXPECTED_SHA256" ]; then
    actual="$(shasum -a 256 "$DEST" | awk '{print $1}')"
    if [ "$actual" = "$EXPECTED_SHA256" ]; then
      echo "skia artifact present and SHA matches: $DEST"
      exit 0
    fi
    echo "skia artifact SHA mismatch (have=$actual want=$EXPECTED_SHA256), re-fetching" >&2
  else
    echo "skia artifact present at $DEST (no SHA pin yet — Phase E will pin)"
    exit 0
  fi
fi

if [ -z "$RELEASE_TAG" ] || [ -z "$RELEASE_FILE" ]; then
  echo "no release tag pinned yet — Phase E will publish; dev builds use skia-bindings from-source path"
  exit 0
fi

RELEASE_URL="https://github.com/ZSeven-W/openpencil/releases/download/${RELEASE_TAG}/${RELEASE_FILE}"
echo "fetching skia artifact: $RELEASE_URL"
curl -fL "$RELEASE_URL" -o "$DEST"

if [ -n "$EXPECTED_SHA256" ]; then
  actual="$(shasum -a 256 "$DEST" | awk '{print $1}')"
  if [ "$actual" != "$EXPECTED_SHA256" ]; then
    echo "skia artifact SHA still mismatched after fetch: have=$actual want=$EXPECTED_SHA256" >&2
    exit 1
  fi
fi
echo "skia artifact fetched: $DEST"
