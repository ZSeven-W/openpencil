#!/bin/bash
# Publish 将所有 @zseven-w 打包到 npm 并自动递增 beta 版本。
#
# Usage：
# Bun runpublish:beta # 自动递增 beta 号
# Bun runPublish:beta 5 # 强制 beta.5
#
# Publishes：笔类型→笔芯→笔 figma→笔渲染器→笔 sdk→openpencil CLI
# All 在“beta”dist 标签下，因此 `npm install` 默认情况下不会选择它们。
# Install 与： npm install @zseven-w/openpencil@beta

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE_VERSION=$(jq -r .version "$ROOT/package.json")
FORCE_NUM="${1:-}"

# --- Guard：如果 npm --- 上已存在发布版本，则阻止 beta 发布
RELEASE_CHECK=$(npm view "@zseven-w/pen-types@${BASE_VERSION}" version 2>/dev/null || true)
if [ -n "$RELEASE_CHECK" ]; then
  echo "ERROR: Release version ${BASE_VERSION} already exists on npm."
  echo "Publishing a beta for an already-released version creates conflicting dependencies."
  echo "Bump the version first (e.g. bun run bump 0.5.2), then publish beta."
  exit 1
fi

# Packages 按拓扑顺序
PACKAGES=(
  packages/pen-types
  packages/pen-core
  packages/pen-figma
  packages/pen-renderer
  packages/pen-mcp
  packages/pen-sdk
  apps/cli
)

# --- Determine 测试版号 ---
if [ -n "$FORCE_NUM" ]; then
  BETA_NUM="$FORCE_NUM"
else
  # Query npm 获取此基础版本的最新测试版。 npm 视图返回一个字符串（1
  # 个版本）或数组（多个），如果未找到，则返回错误 (404)。
  RAW=$(npm view "@zseven-w/pen-types" versions --json 2>/dev/null || true)
  LATEST=$(echo "$RAW" | jq -r --arg base "$BASE_VERSION" '
    if type == "object" and .error then empty          # npm 404 错误对象
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

# 所有 package.json 文件中的 --- Set beta 版本 ---
MODIFIED_FILES=()
for pkg in "${PACKAGES[@]}"; do
  f="$ROOT/$pkg/package.json"
  if [ -f "$f" ]; then
    # Backup 原创
    cp "$f" "$f.bak"
    MODIFIED_FILES+=("$f")

    # Set 版本并替换工作区：* refs
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

# --- Restore 退出时 ---
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
