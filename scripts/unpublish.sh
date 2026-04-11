#!/bin/bash
# Unpublish all @zseven-w packages from npm for a given version.
#
# Usage:
#   bun run unpublish 0.5.1              # unpublish specific version from all packages
#   bun run unpublish 0.5.1 --all        # unpublish ALL versions of all packages
#   bun run unpublish 0.5.1 --deprecate  # deprecate instead of unpublish (fallback)
#
# Packages are removed in reverse topological order (dependents first, then dependencies)
# to avoid npm's "has dependent packages" rejection.

set -euo pipefail

VERSION="${1:-}"
FLAG="${2:-}"

if [ -z "$VERSION" ]; then
  echo "Usage: bun run unpublish <version> [--all|--deprecate]"
  echo ""
  echo "  <version>      Version to unpublish (e.g. 0.5.1)"
  echo "  --all          Unpublish ALL versions of every package"
  echo "  --deprecate    Deprecate instead of unpublish"
  exit 1
fi

# Reverse topological order: dependents first, then dependencies
PACKAGES=(
  @zseven-w/openpencil
  @zseven-w/pen-sdk
  @zseven-w/pen-renderer
  @zseven-w/pen-figma
  @zseven-w/pen-core
  @zseven-w/pen-types
)

FAILED=()

for pkg in "${PACKAGES[@]}"; do
  if [ "$FLAG" = "--deprecate" ]; then
    echo "Deprecating $pkg@$VERSION ..."
    npm deprecate "${pkg}@${VERSION}" "this version has been deprecated, do not use" --force 2>&1 || {
      echo "  ⚠ Failed to deprecate $pkg@$VERSION"
      FAILED+=("$pkg@$VERSION")
    }
  elif [ "$FLAG" = "--all" ]; then
    echo "Unpublishing $pkg (all versions) ..."
    npm unpublish "$pkg" --force 2>&1 || {
      echo "  ⚠ Failed to unpublish $pkg"
      FAILED+=("$pkg")
    }
  else
    echo "Unpublishing $pkg@$VERSION ..."
    npm unpublish "${pkg}@${VERSION}" --force 2>&1 || {
      echo "  ⚠ Failed to unpublish $pkg@$VERSION (try --all to remove entire package)"
      FAILED+=("$pkg@$VERSION")
    }
  fi
  echo ""
done

echo "================================"
if [ ${#FAILED[@]} -eq 0 ]; then
  echo "All packages processed successfully."
else
  echo "Failed packages:"
  for f in "${FAILED[@]}"; do
    echo "  - $f"
  done
  echo ""
  echo "Tip: If unpublish fails due to dependent packages, use --all to remove entire packages,"
  echo "     or --deprecate as a fallback."
fi
echo "================================"
