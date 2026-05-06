import type { PenNode, PathNode } from '@/types/pen';
import { toStrokeThicknessNumber, extractPrimaryColor } from './generation-utils';
import {
  ICON_PATH_MAP,
  findPrefixFallback,
  findSubstringFallback,
  lookupIconByName,
} from './icon-dictionary';
import { pendingIconResolutions, tryImmediateIconResolution } from './icon-font-fetcher';

// ---------------------------------------------------------------------------
// 重新导出：保持现有调用方的公共 API 不变
// ---------------------------------------------------------------------------

export {
  type IconEntry,
  type BuiltinIconEntry,
  ICON_PATH_MAP,
  AVAILABLE_LUCIDE_ICONS,
  AVAILABLE_FEATHER_ICONS,
  BUILTIN_ICONS,
  lookupIconByName,
  findPrefixFallback,
  findSubstringFallback,
} from './icon-dictionary';

export {
  tryAsyncIconFontResolution,
  resolveAsyncIcons,
  resolveAllPendingIcons,
} from './icon-font-fetcher';

export { applyNoEmojiIconHeuristic } from './icon-emoji-heuristics';

// ---------------------------------------------------------------------------
// 图标路径解析：主入口 + 节点属性修正
// ---------------------------------------------------------------------------

/**
 * 用来显式标记“这是图标”的关键字集合。
 *
 * `path` 类型不仅会承载图标，也会承载真正的自定义几何图形，
 * 例如图表线、进度弧、波形、迷你图或插画路径。
 * 所以这里绝对不能对所有 path 节点盲目做图标解析，
 * 否则很容易把真实几何图形误替换成 `circle`、`bar-chart`、`arrow` 一类图标路径。
 *
 * 只有名称里明确表达“这是一个图标”的节点，
 * 才有资格进入后续解析流程。
 */
const ICON_MARKER_WORDS = new Set(['icon', 'logo', 'symbol', 'glyph']);

/**
 * 检查路径节点名称里是否存在显式的图标标记。
 *
 * 它可以处理：
 * - `SearchIcon`（camelCase）
 * - `Search Icon`（空格）
 * - `search_icon`（下划线）
 * - `BrandLogo` / `AppGlyph`
 *
 * 同时会排除“Heart Rate Chart”“Steps Progress”“Chart Fill”
 * 这类描述几何用途的名字。
 */
function hasExplicitIconMarker(name: string): boolean {
  // 先按 camelCase 拆词，再按空格 / 下划线 / 连字符继续切分。
  const words = name
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .toLowerCase()
    .split(/[\s_-]+/);
  for (const word of words) {
    if (ICON_MARKER_WORDS.has(word)) return true;
  }
  return false;
}

/**
 * 根据名称解析图标型 `path` 节点。
 *
 * 如果 AI 生成了像 `SearchIcon`、`MenuIcon` 这样的路径节点，
 * 就从 `ICON_PATH_MAP` 里找到对应的可信 SVG 路径，并替换它的 `d`。
 *
 * 如果本地字典里没有，就先放一个通用占位图标，
 * 再把该节点登记到异步 Iconify 解析流程里。
 *
 * 这里只处理名称中明确带有 `icon` / `logo` / `symbol` / `glyph`
 * 标记的 path 节点。
 * 其余 path 一律视为真实自定义几何图形，保持原样不动。
 *
 * `icon_font` 才是 AI 输出图标的规范节点类型；
 * `icon-resolver` 更像是一层兜底，用来补救 AI 偶尔把图标错误地生成为 `path` 的情况。
 */
export function applyIconPathResolution(node: PenNode): void {
  if (node.type !== 'path') return;

  const originalName = node.name ?? node.id ?? '';
  // 强约束：名称里必须明确出现 icon/logo 标记。
  // 没有这层保护的话，像 “Chart Fill” 这样的名字很容易因为共享子串
  // 被误识别成图标词典里的条目。
  if (!hasExplicitIconMarker(originalName)) return;

  const rawName = originalName
    .toLowerCase()
    .replace(/[-_\s]+/g, '') // 标准化分隔符
    .replace(/(icon|logo|symbol|glyph)$/, ''); // 剥离尾随标记

  let match = ICON_PATH_MAP[rawName];

  if (!match) {
    // 1. 先尝试前缀回退，例如 "arrowdowncircle" -> "arrowdown"
    const prefixKey = findPrefixFallback(rawName);
    if (prefixKey) match = ICON_PATH_MAP[prefixKey];
  }

  if (!match) {
    // 2. 再尝试子串回退，例如 "badgecheck" -> "check"
    const substringKey = findSubstringFallback(rawName);
    if (substringKey) match = ICON_PATH_MAP[substringKey];
  }

  const originalNormalized = (node.name ?? node.id ?? '').toLowerCase().replace(/[-_\s]+/g, '');
  const queueName = rawName || originalNormalized;

  if (!match) {
    // 3. 最后兜底：先放一个通用 Feather 圆形图标，再排队走异步解析。
    if (isIconLikeName(node.name ?? '', queueName) && !isOverlyGenericFallbackName(queueName)) {
      const fallback = ICON_PATH_MAP['circle'] ?? ICON_PATH_MAP['feather:circle'];
      if (fallback) {
        node.d = fallback.d;
        node.iconId = fallback.iconId;
        applyIconStyle(node as import('@/types/pen').PathNode, fallback.style);
      }
      pendingIconResolutions.set(node.id, queueName);
      tryImmediateIconResolution(node.id, queueName);
    }
    return;
  }

  // 用可信路径数据替换，并记录解析出的 iconId
  node.d = match.d;
  node.iconId = match.iconId ?? `feather:${rawName}`;
  applyIconStyle(node, match.style);
}

export function resolveIconPathBySemanticName(node: PathNode, semanticName: string): boolean {
  const match = lookupIconByName(semanticName);
  if (!match) return false;
  node.d = match.d;
  node.iconId = match.iconId;
  applyIconStyle(node, match.style);
  return true;
}

// ---------------------------------------------------------------------------
// 内部辅助函数
// ---------------------------------------------------------------------------

/**
 * 判断一个名字是否像“可解析的图标引用”。
 *
 * 走到这里时，上层已经确认名称带有显式图标标记；
 * 这里额外要求规范化后的名字非空、且长度别太离谱，
 * 这样才值得进入异步 Iconify 解析流程。
 */
function isIconLikeName(_originalName: string, normalized: string): boolean {
  return normalized.length > 0 && normalized.length <= 30;
}

function isOverlyGenericFallbackName(normalized: string): boolean {
  return (
    normalized === 'icon' ||
    /^wc\d+$/.test(normalized) ||
    /^tab[a-z0-9]+$/.test(normalized) ||
    /^nav[a-z0-9]+$/.test(normalized) ||
    /^item\d+$/.test(normalized) ||
    /^section\d+$/.test(normalized)
  );
}

/** Apply stroke/fill styling to a resolved icon node (caller must ensure path type). */
function applyIconStyle(node: PathNode, style: 'stroke' | 'fill'): void {
  if (style === 'stroke') {
    const existingColor =
      extractPrimaryColor('fill' in node ? node.fill : undefined) ??
      extractPrimaryColor(node.stroke?.fill) ??
      '#64748B';
    const strokeWidth = toStrokeThicknessNumber(node.stroke, 0);
    const strokeColor = extractPrimaryColor(node.stroke?.fill);
    // Ensure stroke is renderable for line icons
    if (!node.stroke || strokeWidth <= 0 || !strokeColor) {
      node.stroke = {
        thickness: strokeWidth > 0 ? strokeWidth : 2,
        fill: [{ type: 'solid', color: existingColor }],
      };
    }
    // Line icons should NOT have opaque fill (transparent to show stroke only)
    if (node.fill && node.fill.length > 0) {
      // Move fill color to stroke if stroke has no color
      const fillColor = extractPrimaryColor(node.fill);
      if (fillColor && node.stroke) {
        node.stroke.fill = [{ type: 'solid', color: fillColor }];
      }
      node.fill = [];
    }
  } else {
    // Fill icons must always keep a visible fill.
    const fillColor =
      extractPrimaryColor('fill' in node ? node.fill : undefined) ??
      extractPrimaryColor(node.stroke?.fill) ??
      '#64748B';
    node.fill = [{ type: 'solid', color: fillColor }];
    // Remove non-renderable stroke definitions to avoid transparent-only paths.
    if (node.stroke && toStrokeThicknessNumber(node.stroke, 0) <= 0) {
      node.stroke = undefined;
    }
  }
}
