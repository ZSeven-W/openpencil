import type { PenNode } from '@zseven-w/pen-types';

/**
 * Strip redundant card-style decoration on a frame whose ancestor is
 * already a decorated card.
 *
 * 2026-05-11 user-reported "popular restaurant" mobile design — the LLM
 * built each restaurant row as `role: card` (outer) with cornerRadius +
 * stroke + 2-shadow elevation, then nested an inner `Card Info` frame
 * ALSO with `role: card` + cornerRadius + 2 shadows for the right-side
 * text column. The inner decoration rendered as a visible "border" /
 * card-within-a-card box that the outer card already provided.
 *
 * Element builders (N-tools) emit decoration deterministically without
 * knowing their context — when an LLM places a v1 card builder inside
 * another card, the inner stroke / cornerRadius / shadow stack on top
 * of the outer's, producing the doubled box look. This pass walks the
 * tree and removes the inner decoration when it's redundant with an
 * ancestor's.
 *
 * Strip rule for a frame F:
 *   - F has `stroke` OR `cornerRadius > 0` OR shadow effects, AND
 *   - F's ancestor chain contains another frame with the same kind of
 *     decoration (stroke / cornerRadius / shadow), AND
 *   - F is not a role that legitimately wants decoration even nested
 *     (button, tag, chip, input, search-bar, badge, avatar) — these
 *     elements often live inside cards and need their own affordance
 *
 * Conservative: fills are NOT stripped. A child fill might be an
 * intentional surface change (e.g. dark accent strip inside a white
 * card). stripRedundantSectionFills handles the fill-redundancy
 * heuristic separately.
 */

const KEEP_DECORATION_ROLES = new Set([
  'button',
  'icon-button',
  'fab',
  'tag',
  'chip',
  'badge',
  'status-badge',
  'pill',
  'input',
  'search-bar',
  'form-field',
  'textarea',
  'select',
  'combobox',
  'avatar',
  'avatar-stack',
  'switch',
  'checkbox',
  'radio',
  'toolbar',
  'segmented-control',
]);

interface DecoFlags {
  hasStroke: boolean;
  hasCornerRadius: boolean;
  hasShadow: boolean;
}

function readDecoration(node: PenNode): DecoFlags {
  const n = node as PenNode & {
    stroke?: { thickness?: number };
    cornerRadius?: number | number[];
    effects?: Array<{ type?: string }>;
  };
  const strokeThick = n.stroke?.thickness ?? 0;
  const hasStroke = typeof strokeThick === 'number' && strokeThick > 0;
  const cr = n.cornerRadius;
  const hasCornerRadius =
    typeof cr === 'number' ? cr > 0 : Array.isArray(cr) && cr.some((v) => Number(v) > 0);
  const hasShadow = Array.isArray(n.effects) && n.effects.some((e) => e?.type === 'shadow');
  return { hasStroke, hasCornerRadius, hasShadow };
}

function isRoleProtected(node: PenNode): boolean {
  const role = ((node as { role?: string }).role ?? '').toLowerCase();
  return KEEP_DECORATION_ROLES.has(role);
}

/**
 * Returns true if any node was modified.
 */
export function stripNestedCardDecoration(root: PenNode): boolean {
  let changed = false;
  walk(root, []);
  return changed;

  function walk(node: PenNode, ancestors: PenNode[]): void {
    if (node.type === 'frame') {
      const ancestorDeco = ancestors
        .filter((a) => a.type === 'frame')
        .map(readDecoration)
        .reduce(
          (acc, d) => ({
            hasStroke: acc.hasStroke || d.hasStroke,
            hasCornerRadius: acc.hasCornerRadius || d.hasCornerRadius,
            hasShadow: acc.hasShadow || d.hasShadow,
          }),
          { hasStroke: false, hasCornerRadius: false, hasShadow: false },
        );
      const own = readDecoration(node);
      const isProtected = isRoleProtected(node);
      if (!isProtected) {
        const n = node as PenNode & {
          stroke?: unknown;
          cornerRadius?: unknown;
          effects?: unknown;
        };
        if (own.hasStroke && ancestorDeco.hasStroke) {
          delete n.stroke;
          changed = true;
        }
        if (own.hasCornerRadius && ancestorDeco.hasCornerRadius) {
          delete n.cornerRadius;
          changed = true;
        }
        if (own.hasShadow && ancestorDeco.hasShadow) {
          delete n.effects;
          changed = true;
        }
      }
    }

    if ('children' in node && Array.isArray(node.children)) {
      const nextAncestors = ancestors.concat(node);
      for (const child of node.children) walk(child, nextAncestors);
    }
  }
}
