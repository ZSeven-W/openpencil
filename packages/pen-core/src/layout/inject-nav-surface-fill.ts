import type { PenNode, PenFill, SolidFill } from '@zseven-w/pen-types';

/**
 * Inject a default surface fill on top-level navigation frames that lack
 * one. Sub-agents frequently emit a bottom navigation / top app bar row
 * without an explicit fill — relying on the visual contrast between the
 * cream/tinted page background and the white nav surface to separate the
 * two. But that contrast is what's missing: the nav frame has no fill,
 * so it inherits the parent's transparent backdrop and visually
 * disappears into the cream root background (the food-app brief on
 * 2026-05-04 produced a bottom-nav row of icons floating directly on
 * #FFF8F0, with no surface to anchor them).
 *
 * Scope: ONLY processes direct children of the passed root frame whose
 * role is a navigation role and whose fill is empty/missing. Does NOT
 * recurse, does NOT touch nav frames nested inside cards / sections,
 * and does NOT override an existing fill (sub-agent intent is preserved
 * if it set any fill — even a non-default one — on its own).
 *
 * Returns `true` when any nav frame was patched, so the caller can
 * publish a store update.
 */
const NAV_ROLES = new Set([
  'navbar',
  'nav',
  'tab-bar',
  'bottom-tab-bar',
  'top-nav-bar',
  'top-app-bar',
  'tab-row',
]);

export function injectMissingNavSurfaceFill(rootFrame: PenNode): boolean {
  if (!('children' in rootFrame) || !Array.isArray(rootFrame.children)) return false;

  let changed = false;
  for (const child of rootFrame.children) {
    if (child.type !== 'frame') continue;
    const role = (child as PenNode & { role?: string }).role;
    if (!role || !NAV_ROLES.has(role)) continue;
    const existing = (child as PenNode & { fill?: PenFill[] | string }).fill;
    if (hasAnyFill(existing)) continue;
    (child as PenNode & { fill?: PenFill[] }).fill = [
      { type: 'solid', color: '$color-surface' } as SolidFill,
    ];
    changed = true;
  }
  return changed;
}

/**
 * True when the frame already carries ANY valid fill the renderer would
 * paint — solid OR gradient (linear/radial) OR image. We must NOT
 * overwrite a non-solid fill with our default `$color-surface` solid:
 * sub-agents legitimately put `linear_gradient` on top app bars
 * (sunrise hero gradient), `image` on hero / branded nav surfaces, and
 * `radial_gradient` on splash-style entries. Keep their intent.
 */
function hasAnyFill(fill: PenFill[] | string | undefined): boolean {
  if (!fill) return false;
  if (typeof fill === 'string') return fill.length > 0;
  if (!Array.isArray(fill) || fill.length === 0) return false;
  // Any first entry with a recognized `type` counts as intentional fill.
  const first = fill[0];
  return !!first && typeof (first as { type?: string }).type === 'string';
}
