import type { PenNode } from '@zseven-w/pen-types'

/**
 * Check if a node is a badge/overlay that uses absolute positioning
 * and should not participate in layout flow.
 */
export function isBadgeOverlayNode(node: PenNode): boolean {
  if ('role' in node) {
    const role = (node as { role?: string }).role
    if (role === 'badge' || role === 'pill' || role === 'tag') return true
  }
  const name = (node.name ?? '').toLowerCase()
  return /badge|indicator|notification[-_\s]?dot|overlay|floating/i.test(name)
}
