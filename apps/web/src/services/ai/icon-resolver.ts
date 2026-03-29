import type { PenNode, PathNode } from '@/types/pen';
import { toStrokeThicknessNumber, extractPrimaryColor } from './generation-utils';
import {
  ICON_PATH_MAP,
  findPrefixFallback,
  findSubstringFallback,
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
 * Resolve icon path nodes by their name. When the AI generates a path node
 * with a name like "SearchIcon" or "MenuIcon", look up the verified SVG path
 * from ICON_PATH_MAP and replace the d attribute.
 *
 * On local map miss for icon-like names, sets a generic placeholder and
 * records the node for async resolution via the Iconify API.
 */
export function applyIconPathResolution(node: PenNode): void {
  if (node.type !== 'path') return;
  const rawName = (node.name ?? node.id ?? '')
    .toLowerCase()
    .replace(/[-_\s]+/g, '') // normalize separators
    .replace(/(icon|logo)$/, ''); // strip trailing "icon" or "logo"

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
    if (isIconLikeName(node.name ?? '', queueName)) {
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

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Check if a name looks like an icon reference (not just any path node). */
function isIconLikeName(originalName: string, normalized: string): boolean {
  // Explicit icon/logo suffix in original name
  if (/icon|logo/i.test(originalName)) return true;
  // Short normalized name (likely an icon name, not a complex path description)
  if (normalized.length > 0 && normalized.length <= 30) return true;
  return false;
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
