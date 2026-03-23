#!/bin/bash
# Publish all @zseven-w packages to npm with auto-incrementing beta version.
#
# Usage:
#   bun run publish:beta          # auto-increment beta number
#   bun run publish:beta 5        # force beta.5
#
# Publishes: pen-types → pen-core → pen-codegen, pen-figma → pen-renderer → pen-sdk → openpencil CLI
# All under the "beta" dist-tag, so `npm install` won't pick them up by default.
# Install with: npm install @zseven-w/openpencil@beta

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE_VERSION=$(jq -r .version "$ROOT/package.json")
FORCE_NUM="${1:-}"

# Packages in topological order
PACKAGES=(
  packages/pen-types
  packages/pen-core
  packages/pen-codegen
  packages/pen-figma
  packages/pen-renderer
  packages/pen-sdk
  apps/cli
)

# --- Determine beta number ---
if [ -n "$FORCE_NUM" ]; then
  BETA_NUM="$FORCE_NUM"
else
  # Query npm for the latest beta of this base version.
  # npm view returns a string (1 version) or array (multiple), or errors (404) if not found.
  RAW=$(npm view "@zseven-w/pen-types" versions --json 2>/dev/null || true)
  LATEST=$(echo "$RAW" | jq -r --arg base "$BASE_VERSION" '
    if type == "object" and .error then empty          # npm 404 error object
    elif type == "array" then
      map(select(type == "string" and startswith($base + "-beta."))) | last // empty
    elif type == "string" and startswith($base + "-beta.") then .
    else empty
    end
  ' 2>/dev/null || true)

  if [ -n "$LATEST" ]; then
    PREV_NUM=$(echo "$LATEST" | sed "s/${BASE_VERSION}-beta\.//")
    BETA_NUM=$((PREV_NUM + 1))
  else
    BETA_NUM=0
  fi
fi

BETA_VERSION="${BASE_VERSION}-beta.${BETA_NUM}"
echo "Publishing version: $BETA_VERSION"
echo ""

# --- Set beta version in all package.json files ---
MODIFIED_FILES=()
for pkg in "${PACKAGES[@]}"; do
  f="$ROOT/$pkg/package.json"
  if [ -f "$f" ]; then
    # Backup original
    cp "$f" "$f.bak"
    MODIFIED_FILES+=("$f")

    # Set version and replace workspace:* refs
    jq --arg v "$BETA_VERSION" '
      .version = $v |
      if .dependencies then
        .dependencies |= with_entries(
          if .value == "workspace:*" then .value = $v else . end
        )
      else . end |
      if .devDependencies then
        .devDependencies |= with_entries(
          if .value == "workspace:*" then .value = $v else . end
        )
      else . end
    ' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
  fi
done

# --- Restore on exit ---
cleanup() {
  echo ""
  echo "Restoring original package.json files..."
  for f in "${MODIFIED_FILES[@]}"; do
    if [ -f "$f.bak" ]; then
      mv "$f.bak" "$f"
    fi
  done
  echo "Done."
}
trap cleanup EXIT

# --- Compile CLI ---
echo "Compiling CLI..."
(cd "$ROOT" && bun run cli:compile)
echo ""

# --- Verify CLI ---
node "$ROOT/apps/cli/dist/openpencil-cli.cjs" --version
echo ""

# --- Publish ---
for pkg in "${PACKAGES[@]}"; do
  dir="$ROOT/$pkg"
  name=$(jq -r .name "$dir/package.json")
  echo "Publishing $name@$BETA_VERSION ..."
  (cd "$dir" && npm publish --access public --tag beta) || echo "  ⚠ Failed (may already exist)"
  echo ""
done

echo "================================"
echo "Published: $BETA_VERSION"
echo "Install:   npm install @zseven-w/openpencil@beta"
echo "================================"
