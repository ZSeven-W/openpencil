#!/bin/bash
# Unpublish 来自 npm 的给定版本的所有 @zseven-w 软件包。
#
# Usage：
# Bun run unpublish 0.5.1 # 从所有包中取消发布特定版本
# Bun run unpublish 0.5.1 --all # 取消发布所有软件包的 ALL 版本
# Bun run unpublish 0.5.1 --deprecate # 弃用而不是取消发布（后备）
#
# Packages 按逆拓扑顺序删除（首先是依赖项，然后是依赖项）
# 避免 npm 的“有依赖包”拒绝。

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

# Reverse 拓扑顺序：首先是依赖项，然后是依赖项
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
if [ ${#FAILED[@]} -eq 0 ];然后
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
