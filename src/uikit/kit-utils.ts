import type { PenDocument, PenNode } from '@/types/pen'
import type { ComponentCategory, KitComponent } from '@/types/uikit'

/**
 * Walk the document tree and extract all reusable nodes as KitComponent metadata.
 */
export function extractComponentsFromDocument(
  doc: PenDocument,
  metaOverrides?: Record<string, { category: ComponentCategory; tags: string[] }>,
): KitComponent[] {
  const components: KitComponent[] = []
  walkTree(doc.children, (node) => {
    if ('reusable' in node && node.reusable) {
      const w = 'width' in node && typeof node.width === 'number' ? node.width : 100
      const h = 'height' in node && typeof node.height === 'number' ? node.height : 100
      const meta = metaOverrides?.[node.id]
      components.push({
        id: node.id,
        name: node.name ?? node.id,
        category: meta?.category ?? inferCategory(node.name ?? ''),
        tags: meta?.tags ?? inferTags(node.name ?? ''),
        width: w,
        height: h,
      })
    }
  })
  return components
}

/**
 * Find a specific node in a document tree by ID.
 */
export function findReusableNode(doc: PenDocument, nodeId: string): PenNode | undefined {
  return findInTree(doc.children, nodeId)
}

/**
 * Recursively collect all $variable references used by a node tree.
 */
export function collectVariableRefs(node: PenNode): Set<string> {
  const refs = new Set<string>()
  collectRefs(node, refs)
  return refs
}

/**
 * Deep-clone a node tree preserving all IDs.
 */
export function deepCloneNode<T extends PenNode>(node: T): T {
  return structuredClone(node)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function walkTree(nodes: PenNode[], visitor: (node: PenNode) => void): void {
  for (const node of nodes) {
    visitor(node)
    if ('children' in node && node.children) {
      walkTree(node.children, visitor)
    }
  }
}

function findInTree(nodes: PenNode[], id: string): PenNode | undefined {
  for (const node of nodes) {
    if (node.id === id) return node
    if ('children' in node && node.children) {
      const found = findInTree(node.children, id)
      if (found) return found
    }
  }
  return undefined
}

function isVariableRef(value: unknown): value is string {
  return typeof value === 'string' && value.startsWith('$')
}

function collectRefs(node: PenNode, refs: Set<string>): void {
  if (isVariableRef(node.opacity)) refs.add(node.opacity)
  if (isVariableRef(node.enabled)) refs.add(node.enabled)

  if ('gap' in node && isVariableRef(node.gap)) refs.add(node.gap as string)
  if ('padding' in node && isVariableRef(node.padding)) refs.add(node.padding as string)

  if ('fill' in node && Array.isArray(node.fill)) {
    for (const f of node.fill) {
      if (f.type === 'solid' && isVariableRef(f.color)) refs.add(f.color)
    }
  }

  if ('stroke' in node && node.stroke) {
    const s = node.stroke
    if (Array.isArray(s.fill)) {
      for (const f of s.fill) {
        if (f.type === 'solid' && isVariableRef(f.color)) refs.add(f.color)
      }
    }
    if (isVariableRef(s.thickness)) refs.add(s.thickness as unknown as string)
  }

  if ('children' in node && node.children) {
    for (const child of node.children) {
      collectRefs(child, refs)
    }
  }
}

const CATEGORY_KEYWORDS: Record<ComponentCategory, string[]> = {
  buttons: ['button', 'btn', 'cta'],
  inputs: ['input', 'text field', 'textarea', 'checkbox', 'radio', 'toggle', 'switch', 'select', 'form'],
  cards: ['card', 'tile'],
  navigation: ['nav', 'navbar', 'tab', 'breadcrumb', 'menu', 'sidebar'],
  layout: ['divider', 'separator', 'spacer', 'container', 'grid'],
  feedback: ['alert', 'banner', 'toast', 'badge', 'tag', 'chip', 'avatar', 'tooltip'],
  'data-display': ['table', 'list', 'stat', 'chart', 'progress'],
  other: [],
}

function inferCategory(name: string): ComponentCategory {
  const lower = name.toLowerCase()
  for (const [category, keywords] of Object.entries(CATEGORY_KEYWORDS)) {
    if (category === 'other') continue
    for (const kw of keywords) {
      if (lower.includes(kw)) return category as ComponentCategory
    }
  }
  return 'other'
}

function inferTags(name: string): string[] {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .split(/[\s-]+/)
    .filter(Boolean)
}
