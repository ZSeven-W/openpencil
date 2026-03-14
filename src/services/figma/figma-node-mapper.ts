import type { FigmaDecodedFile, FigmaImportLayoutMode, FigmaNodeChange } from './figma-types'
import type { PenNode, PenPage, PenDocument } from '@/types/pen'
import {
  type TreeNode,
  guidToString,
  isUserPage,
  buildTree,
  buildTreeForClipboard,
  collectComponents,
  collectSymbolTree,
} from './figma-tree-builder'
import {
  type ConversionContext,
  convertChildren,
  convertNode,
  collectImageBlobs,
} from './figma-node-converters'

/**
 * Resolve styleIdForFill / styleIdForStrokeFill references to inline paints.
 * Figma stores paint styles as separate nodes (styleType='FILL') and references
 * them via styleIdForFill on consuming nodes.  Nodes with a style ref but no
 * inline fillPaints need the style's paints copied in.
 */
function resolveStyleReferences(nodeChanges: FigmaNodeChange[]): void {
  // Build style map from nodes with styleType
  const styleMap = new Map<string, FigmaNodeChange>()
  for (const nc of nodeChanges) {
    if ((nc as any).styleType && nc.guid) {
      styleMap.set(guidToString(nc.guid), nc)
    }
  }
  if (styleMap.size === 0) return

  /** Resolve style references on a single node-like object. */
  function resolveOnNode(nc: Record<string, any>) {
    // Resolve fill style — always use the style's paint when a style reference exists
    const fillStyleId = nc.styleIdForFill as { guid?: { sessionID: number; localID: number } } | undefined
    if (fillStyleId?.guid) {
      const styleKey = `${fillStyleId.guid.sessionID}:${fillStyleId.guid.localID}`
      const style = styleMap.get(styleKey)
      if (style?.fillPaints?.length) {
        nc.fillPaints = style.fillPaints
      }
    }
    // Resolve stroke fill style
    const strokeStyleId = nc.styleIdForStrokeFill as { guid?: { sessionID: number; localID: number } } | undefined
    if (strokeStyleId?.guid) {
      const styleKey = `${strokeStyleId.guid.sessionID}:${strokeStyleId.guid.localID}`
      const style = styleMap.get(styleKey)
      if (style?.fillPaints?.length) {
        nc.strokePaints = style.fillPaints
      }
    }
  }

  for (const nc of nodeChanges) {
    resolveOnNode(nc as Record<string, any>)
    // Also resolve style references inside instance override entries
    const overrides = nc.symbolData?.symbolOverrides
    if (overrides) {
      for (const ov of overrides) {
        resolveOnNode(ov as Record<string, any>)
      }
    }
  }
}

/**
 * Convert a decoded .fig file to a PenDocument.
 */
export function figmaToPenDocument(
  decoded: FigmaDecodedFile,
  fileName: string,
  pageIndex: number = 0,
  layoutMode: FigmaImportLayoutMode = 'openpencil',
): { document: PenDocument; warnings: string[]; imageBlobs: Map<number, Uint8Array> } {
  const warnings: string[] = []

  // Resolve style references before tree building
  resolveStyleReferences(decoded.nodeChanges)

  const tree = buildTree(decoded.nodeChanges)

  if (!tree) {
    return {
      document: { version: '1', name: fileName, children: [] },
      warnings: ['No document root found'],
      imageBlobs: new Map(),
    }
  }

  const pages = tree.children.filter(isUserPage)
  const page = pages[pageIndex] ?? pages[0]

  if (!page) {
    return {
      document: { version: '1', name: fileName, children: [] },
      warnings: ['No pages found in Figma file'],
      imageBlobs: new Map(),
    }
  }

  const componentMap = new Map<string, string>()
  const symbolTree = new Map<string, TreeNode>()
  let idCounter = 1
  collectComponents(page, componentMap, () => `fig_${idCounter++}`)
  // Collect SYMBOL tree nodes from ALL canvases (including Figma's internal canvas
  // where master components live) so INSTANCE nodes can inline their content.
  collectSymbolTree(tree, symbolTree)

  const ctx: ConversionContext = {
    componentMap,
    symbolTree,
    warnings,
    generateId: () => `fig_${idCounter++}`,
    blobs: decoded.blobs,
    layoutMode,
  }

  const children = convertChildren(page, ctx)
  const imageBlobs = collectImageBlobs(decoded.blobs)

  const pageName = page.figma.name ?? 'Page 1'
  const penPage: PenPage = {
    id: `figma-page-${pageIndex}`,
    name: pageName,
    children,
  }

  return {
    document: {
      version: '1',
      name: fileName,
      pages: [penPage],
      children: [],
    },
    warnings,
    imageBlobs,
  }
}

/**
 * Convert ALL pages from a decoded .fig file into a single PenDocument.
 * Each page's children are placed side by side with a horizontal gap.
 */
export function figmaAllPagesToPenDocument(
  decoded: FigmaDecodedFile,
  fileName: string,
  layoutMode: FigmaImportLayoutMode = 'openpencil',
): { document: PenDocument; warnings: string[]; imageBlobs: Map<number, Uint8Array> } {
  const warnings: string[] = []

  resolveStyleReferences(decoded.nodeChanges)

  const tree = buildTree(decoded.nodeChanges)
  if (!tree) {
    return {
      document: { version: '1', name: fileName, children: [] },
      warnings: ['No document root found'],
      imageBlobs: new Map(),
    }
  }

  const allCanvases = tree.children.filter((c) => c.figma.type === 'CANVAS')
  const pages = allCanvases.filter(isUserPage)
  if (pages.length === 0) {
    return {
      document: { version: '1', name: fileName, children: [] },
      warnings: ['No pages found in Figma file'],
      imageBlobs: new Map(),
    }
  }

  const componentMap = new Map<string, string>()
  const symbolTree = new Map<string, TreeNode>()
  let idCounter = 1
  const genId = () => `fig_${idCounter++}`
  // Only collect components from user-visible pages so that SYMBOL masters
  // living on Figma's internal canvas don't get registered.  When an INSTANCE
  // references a SYMBOL that isn't in componentMap, convertInstance will
  // inline the master's children via symbolTree instead of emitting a
  // dangling ref node.
  for (const page of pages) {
    collectComponents(page, componentMap, genId)
  }
  collectSymbolTree(tree, symbolTree)

  const penPages: PenPage[] = []

  for (let i = 0; i < pages.length; i++) {
    const page = pages[i]
    const ctx: ConversionContext = {
      componentMap,
      symbolTree,
      warnings,
      generateId: genId,
      blobs: decoded.blobs,
      layoutMode,
    }

    const pageChildren = convertChildren(page, ctx)
    const pageName = page.figma.name ?? `Page ${i + 1}`

    penPages.push({
      id: `figma-page-${i}`,
      name: pageName,
      children: pageChildren,
    })
  }

  const imageBlobs = collectImageBlobs(decoded.blobs)

  return {
    document: {
      version: '1',
      name: fileName,
      pages: penPages,
      children: [],
    },
    warnings,
    imageBlobs,
  }
}

/**
 * Get pages from a decoded .fig file.
 */
export function getFigmaPages(
  decoded: FigmaDecodedFile
): { id: string; name: string; childCount: number }[] {
  const tree = buildTree(decoded.nodeChanges)
  if (!tree) return []

  return tree.children
    .filter(isUserPage)
    .map((c) => ({
      id: guidToString(c.figma.guid!),
      name: c.figma.name ?? 'Page',
      childCount: c.children.length,
    }))
}

/**
 * Convert decoded Figma nodeChanges directly to PenNodes (without wrapping in a PenDocument).
 * Used for clipboard paste where the data may lack a DOCUMENT+CANVAS wrapper.
 */
export function figmaNodeChangesToPenNodes(
  decoded: FigmaDecodedFile,
  layoutMode: FigmaImportLayoutMode = 'openpencil',
): { nodes: PenNode[]; warnings: string[]; imageBlobs: Map<number, Uint8Array> } {
  const warnings: string[] = []

  resolveStyleReferences(decoded.nodeChanges)

  const tree = buildTree(decoded.nodeChanges)
  let topNodes: TreeNode[]

  if (tree) {
    const pages = tree.children.filter(isUserPage)
    const page = pages[0]
    if (page) {
      topNodes = page.children
    } else if (tree.children.length > 0) {
      topNodes = tree.children
    } else {
      topNodes = []
    }
  } else {
    topNodes = buildTreeForClipboard(decoded.nodeChanges)
  }

  if (topNodes.length === 0) {
    return { nodes: [], warnings: ['No convertible nodes found'], imageBlobs: new Map() }
  }

  const componentMap = new Map<string, string>()
  const symbolTree = new Map<string, TreeNode>()
  let idCounter = 1
  const genId = () => `fig_${idCounter++}`
  for (const node of topNodes) {
    collectComponents(node, componentMap, genId)
  }
  // For clipboard, also scan all available nodes for symbols
  if (tree) collectSymbolTree(tree, symbolTree)
  for (const node of topNodes) collectSymbolTree(node, symbolTree)

  const ctx: ConversionContext = {
    componentMap,
    symbolTree,
    warnings,
    generateId: genId,
    blobs: decoded.blobs,
    layoutMode,
  }

  const nodes: PenNode[] = []
  for (const treeNode of topNodes) {
    if (treeNode.figma.visible === false) continue
    const node = convertNode(treeNode, undefined, ctx)
    if (node) nodes.push(node)
  }

  const imageBlobs = collectImageBlobs(decoded.blobs)

  return { nodes, warnings, imageBlobs }
}
