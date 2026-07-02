import type { PenNode } from '@zseven-w/pen-types';

/**
 * Check if a node is an overlay that uses absolute positioning and should
 * not participate in layout flow.
 *
 * Requires explicit `role: 'overlay'`. Earlier versions matched on
 * `role: 'badge' | 'pill' | 'tag'` plus name regexes, but those are
 * inline-component markers in this repo (see `role-resolver.ts` and
 * `strip-redundant-section-fills.ts` PROTECTED_ROLES) — pulling them out
 * of layout flow collapsed them to (0,0) of their parent and stacked
 * them on top of siblings. `role: 'overlay'` is the dedicated opt-in for
 * notification dots and true floating decorations.
 */
export function isOverlayNode(node: PenNode): boolean {
  if ('role' in node) {
    const role = (node as { role?: string }).role;
    if (role === 'overlay') return true;
  }
  return false;
}

/**
 * @deprecated Renamed to `isOverlayNode`. Semantics also tightened:
 * this alias no longer returns true for `role: 'badge' | 'pill' | 'tag'`
 * (those are inline-component roles in this repo and should flow in
 * auto-layout, not float). Use `isOverlayNode` and mark true floating
 * decorations with `role: 'overlay'`.
 */
export const isBadgeOverlayNode = isOverlayNode;

/**
 * Convert a name string to PascalCase.
 * Strips non-alphanumeric characters and joins words.
 */
export function sanitizeName(name: string): string {
  return name
    .replace(/[^a-zA-Z0-9\s-_]/g, '')
    .split(/[\s\-_]+/)
    .filter(Boolean)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join('');
}
