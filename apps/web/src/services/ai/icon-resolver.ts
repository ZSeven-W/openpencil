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
// Re-exports — keep the public API surface unchanged for existing consumers
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
// Icon path resolution — main entry point + node property mutation
// ---------------------------------------------------------------------------

/**
 * Reserved words that mark a path node as explicitly an icon/logo/symbol.
 * The `path` type is also used for legitimate custom geometry (chart
 * lines, progress arcs, waveforms, sparklines, illustrations), so we
 * MUST NOT blindly run icon resolution on every path node — that would
 * clobber the real geometry with a circle/bar-chart/arrow icon path.
 *
 * Only names that clearly signal "this is an icon" are candidates —
 * tested by splitting the name into words on camelCase, spaces, dashes
 * and underscores, then checking for an exact word hit.
 */
const ICON_MARKER_WORDS = new Set(['icon', 'logo', 'symbol', 'glyph']);

/**
 * Check whether a path node's name carries an explicit icon marker.
 * Handles "SearchIcon" (camelCase), "Search Icon" (spaced), "search_icon"
 * (snake), "search-icon" (kebab), and "BrandLogo" / "AppGlyph".
 * Rejects descriptive geometry names like "Heart Rate Chart",
 * "Steps Progress", "Chart Fill", "Heart Rate Waveform".
 */
function hasExplicitIconMarker(name: string): boolean {
  // Split on camelCase boundaries, then on whitespace/underscore/hyphen.
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
 * Resolve icon path nodes by their name. When the AI generates a path node
 * with a name like "SearchIcon" or "MenuIcon", look up the verified SVG path
 * from ICON_PATH_MAP and replace the d attribute.
 *
 * On local map miss for icon-like names, sets a generic placeholder and
 * records the node for async resolution via the Iconify API.
 *
 * IMPORTANT: Only path nodes whose name explicitly says "icon"/"logo"/
 * "symbol"/"glyph" are considered. Everything else is treated as real
 * custom geometry and left alone — AI-generated data-viz paths like
 * "Heart Rate Chart", "Steps Progress", "Chart Fill" must never be
 * hijacked into a circle or bar-chart icon. The `icon_font` node type
 * is the canonical way for AI to emit icons; icon_resolver only exists
 * to salvage the rare case where AI picks `path` but still means an icon.
 */
export function applyIconPathResolution(node: PenNode): void {
  if (node.type !== 'path') return;

  const originalName = node.name ?? node.id ?? '';
  // Hard gate: require an explicit icon/logo marker in the name.
  // Without this guard, descriptive path names share substrings with icon
  // dictionary keys (e.g. "Chart Fill" → prefix "chart") and get
  // overwritten with the matched icon path.
  if (!hasExplicitIconMarker(originalName)) return;

  const rawName = originalName
    .toLowerCase()
    .replace(/[-_\s]+/g, '') // normalize separators
    .replace(/(icon|logo|symbol|glyph)$/, ''); // strip trailing marker

  let match = ICON_PATH_MAP[rawName];

  if (!match) {
    // 1. Try prefix fallback: "arrowdowncircle" -> "arrowdown", "shieldcheck" -> "shield"
    const prefixKey = findPrefixFallback(rawName);
    if (prefixKey) match = ICON_PATH_MAP[prefixKey];
  }

  if (!match) {
    // 2. Try substring fallback: "badgecheck" -> "check", "uploadcloud" -> "upload"
    const substringKey = findSubstringFallback(rawName);
    if (substringKey) match = ICON_PATH_MAP[substringKey];
  }

  const originalNormalized = (node.name ?? node.id ?? '').toLowerCase().replace(/[-_\s]+/g, '');
  const queueName = rawName || originalNormalized;

  if (!match) {
    // 3. Last resort: circle from Feather, queued for async.
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

  // Replace with verified path data and mark as resolved icon
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
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Check if a name looks like an icon reference (not just any path node).
 *
 * The top-level guard in applyIconPathResolution already requires an
 * explicit icon/logo/symbol/glyph marker, so by the time we get here we
 * know the caller believes this is an icon. We still want a short
 * non-empty normalized form so we can queue it for async Iconify
 * resolution (empty after normalization means there is nothing to look up).
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
