import type { PenNode } from '@zseven-w/pen-types';

/**
 * Check if a node is a badge/overlay that uses absolute positioning
 * and should not participate in layout flow.
 */
export function isBadgeOverlayNode(node: PenNode): boolean {
  if ('role' in node) {
    const role = (node as { role?: string }).role;
    if (role === 'badge' || role === 'pill' || role === 'tag') return true;
  }
  const name = (node.name ?? '').toLowerCase();
  return /badge|indicator|notification[-_\s]?dot|overlay|floating/i.test(name);
}

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
