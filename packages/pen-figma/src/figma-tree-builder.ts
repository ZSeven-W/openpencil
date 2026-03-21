import type { FigmaNodeChange, FigmaGUID } from './figma-types'

export interface TreeNode {
  figma: FigmaNodeChange
  children: TreeNode[]
}

export function guidToString(guid: FigmaGUID): string {
  return `${guid.sessionID}:${guid.localID}`
}

/** Filter out Figma's internal-only canvas (stores metadata, not user content). */
export function isUserPage(node: TreeNode): boolean {
  return node.figma.type === 'CANVAS' &&
    !/^Internal\s+Only/i.test(node.figma.name ?? '')
}

export function buildTree(nodeChanges: FigmaNodeChange[]): TreeNode | null {
  const nodeMap = new Map<string, TreeNode>()
  let root: TreeNode | null = null

  for (const nc of nodeChanges) {
    if (!nc.guid) continue
    if (nc.phase === 'REMOVED') continue
    const key = guidToString(nc.guid)
    nodeMap.set(key, { figma: nc, children: [] })
  }

  for (const nc of nodeChanges) {
    if (!nc.guid || nc.phase === 'REMOVED') continue
    const key = guidToString(nc.guid)
    const treeNode = nodeMap.get(key)
    if (!treeNode) continue

    if (nc.type === 'DOCUMENT') {
      root = treeNode
      continue
    }

    if (nc.parentIndex?.guid) {
      const parentKey = guidToString(nc.parentIndex.guid)
      const parent = nodeMap.get(parentKey)
      if (parent) {
        parent.children.push(treeNode)
      }
    }
  }

  if (root) {
    sortChildrenRecursive(root)
  }

  return root
}

/**
 * Build a tree from clipboard nodeChanges that may lack a DOCUMENT wrapper.
 * Collects orphan nodes (whose parent is not in the data) as roots.
 */
export function buildTreeForClipboard(nodeChanges: FigmaNodeChange[]): TreeNode[] {
  const nodeMap = new Map<string, TreeNode>()
  const childKeys = new Set<string>()

  for (const nc of nodeChanges) {
    if (!nc.guid) continue
    if (nc.phase === 'REMOVED') continue
    const key = guidToString(nc.guid)
    nodeMap.set(key, { figma: nc, children: [] })
  }

  for (const nc of nodeChanges) {
    if (!nc.guid || nc.phase === 'REMOVED') continue
    const key = guidToString(nc.guid)
    const treeNode = nodeMap.get(key)
    if (!treeNode) continue

    if (nc.parentIndex?.guid) {
      const parentKey = guidToString(nc.parentIndex.guid)
      const parent = nodeMap.get(parentKey)
      if (parent) {
        parent.children.push(treeNode)
        childKeys.add(key)
      }
    }
  }

  const roots: TreeNode[] = []
  for (const [key, node] of nodeMap) {
    if (!childKeys.has(key) && node.figma.type !== 'DOCUMENT') {
      roots.push(node)
    }
  }

  for (const root of roots) {
    sortChildrenRecursive(root)
  }

  return roots
}

function sortChildrenRecursive(node: TreeNode): void {
  node.children.sort((a, b) => {
    const posA = a.figma.parentIndex?.position ?? ''
    const posB = b.figma.parentIndex?.position ?? ''
    return posA < posB ? 1 : posA > posB ? -1 : 0
  })
  for (const child of node.children) {
    sortChildrenRecursive(child)
  }
}

export function collectComponents(
  node: TreeNode,
  map: Map<string, string>,
  genId: () => string,
): void {
  if (node.figma.type === 'SYMBOL' && node.figma.guid) {
    const figmaId = guidToString(node.figma.guid)
    map.set(figmaId, genId())
  }
  for (const child of node.children) {
    collectComponents(child, map, genId)
  }
}

/** Collect SYMBOL TreeNodes keyed by figma GUID from all canvases (including internal). */
export function collectSymbolTree(
  root: TreeNode,
  map: Map<string, TreeNode>,
): void {
  if (root.figma.type === 'SYMBOL' && root.figma.guid) {
    map.set(guidToString(root.figma.guid), root)
  }
  for (const child of root.children) {
    collectSymbolTree(child, map)
  }
}
