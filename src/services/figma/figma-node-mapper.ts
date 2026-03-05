import type { FigmaDecodedFile, FigmaImportLayoutMode } from './figma-types'
import type { PenNode, PenPage, PenDocument } from '@/types/pen'
import {
  type TreeNode,
  guidToString,
  isUserPage,
  buildTree,
  buildTreeForClipboard,
  collectComponents,
} from './figma-tree-builder'
import {
  type ConversionContext,
  convertChildren,
  convertNode,
  collectImageBlobs,
} from './figma-node-converters'

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
  let idCounter = 1
  collectComponents(page, componentMap, () => `fig_${idCounter++}`)

  const ctx: ConversionContext = {
    componentMap,
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
  let idCounter = 1
  const genId = () => `fig_${idCounter++}`
  for (const page of allCanvases) {
    collectComponents(page, componentMap, genId)
  }

  const penPages: PenPage[] = []

  for (let i = 0; i < pages.length; i++) {
    const page = pages[i]
    const ctx: ConversionContext = {
      componentMap,
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
  let idCounter = 1
  const genId = () => `fig_${idCounter++}`
  for (const node of topNodes) {
    collectComponents(node, componentMap, genId)
  }

  const ctx: ConversionContext = {
    componentMap,
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
