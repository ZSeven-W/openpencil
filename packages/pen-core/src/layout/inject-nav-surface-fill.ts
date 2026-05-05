import type { PenNode, PenFill, PenEffect, ShadowEffect, SolidFill } from '@zseven-w/pen-types';

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

// Roles that sit at the BOTTOM of the screen — their shadow points
// up so the nav lifts off the content above. Anything else (top nav
// bar, generic navbar) gets a downward shadow lifting it off the
// content below.
const BOTTOM_NAV_ROLES = new Set(['bottom-tab-bar']);

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

    // Why also inject a shadow: in warm-light themes (`$color-bg-deep`
    // = #FFF8F0 cream, `$color-surface` = #FFFFFF white), the
    // luminance delta between page bg and the surface fill we just
    // applied is ~0.03 — visually indistinguishable. The user reads
    // the nav as having "no background" even though it does. Adding
    // a soft upward shadow lifts the nav off the page bg
    // independently of the fill contrast. We only add the shadow
    // when no `effects` were already set; if the sub-agent emitted
    // its own effects (intentional drop shadow, brand glow, etc.)
    // we leave them alone.
    const existingEffects = (child as PenNode & { effects?: PenEffect[] }).effects;
    const hasEffects = Array.isArray(existingEffects) && existingEffects.length > 0;
    if (!hasEffects) {
      // Bottom nav: shadow above (offsetY < 0) — lifts off content
      // above. Top nav / generic navbar: shadow below (offsetY > 0)
      // — lifts off content below. A downward shadow on a bottom
      // nav would hide off-screen and not provide any separation,
      // and an upward shadow on a top nav would cling to the screen
      // edge and look broken.
      const isBottomNav = BOTTOM_NAV_ROLES.has(role);
      const shadow: ShadowEffect = {
        type: 'shadow',
        offsetX: 0,
        offsetY: isBottomNav ? -4 : 4,
        blur: 12,
        spread: 0,
        color: '#0000000F',
      };
      (child as PenNode & { effects?: PenEffect[] }).effects = [shadow];
    }
    changed = true;
  }
  return changed;
}

/**
 * True when the frame carries a fill the renderer can ACTUALLY paint —
 * solid with a non-empty color, gradient with stops, or image with a
 * src. A truthy `type` field alone isn't enough: sub-agents sometimes
 * emit `[{type:'solid'}]` (missing color), `[{type:'solid',color:''}]`
 * (empty color), or `[{type:'invalid'}]` (unknown variant). Those
 * frames render as transparent, so they're effectively unfilled and
 * the inject pass should still patch them. Conversely, real
 * `linear_gradient` / `radial_gradient` / `image` fills with the
 * required fields are intentional and must be preserved.
 */
function hasAnyFill(fill: PenFill[] | string | undefined): boolean {
  if (!fill) return false;
  if (typeof fill === 'string') return fill.length > 0;
  if (!Array.isArray(fill) || fill.length === 0) return false;
  const first = fill[0] as unknown as Record<string, unknown> | undefined;
  if (!first || typeof first.type !== 'string') return false;
  switch (first.type) {
    case 'solid':
      return typeof first.color === 'string' && first.color.length > 0;
    case 'linear_gradient':
    case 'radial_gradient':
      return Array.isArray(first.stops) && first.stops.length > 0;
    case 'image':
      return typeof first.src === 'string' && first.src.length > 0;
    default:
      // Unknown fill type — renderer can't paint it, treat as unfilled
      // so the inject pass adds a default surface.
      return false;
  }
}
