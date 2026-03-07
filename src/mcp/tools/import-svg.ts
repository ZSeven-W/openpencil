import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { openDocument, saveDocument, resolveDocPath } from '../document-manager'
import {
  insertNodeInTree,
  getDocChildren,
  setDocChildren,
  flattenNodes,
} from '../utils/node-operations'
import { parseSvgToNodesServer } from '../utils/svg-node-parser'
import { postProcessNode } from './node-crud'

export interface ImportSvgParams {
  filePath?: string
  svgPath: string
  parent?: string | null
  maxDim?: number
  postProcess?: boolean
  canvasWidth?: number
  pageId?: string
}

export async function handleImportSvg(
  params: ImportSvgParams,
): Promise<{ nodeIds: string[]; nodeCount: number; postProcessed?: boolean }> {
  // Read SVG file
  const svgAbsPath = resolve(params.svgPath)
  const svgText = await readFile(svgAbsPath, 'utf-8')

  // Parse SVG into PenNodes
  const nodes = parseSvgToNodesServer(svgText, params.maxDim ?? 400)
  if (nodes.length === 0) {
    throw new Error(`No parseable elements found in SVG: ${svgAbsPath}`)
  }

  // Open target document and insert nodes
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)
  const pageId = params.pageId

  const parent = params.parent ?? null
  let children = getDocChildren(doc, pageId)
  for (const node of nodes) {
    children = insertNodeInTree(children, parent, node)
  }
  setDocChildren(doc, children, pageId)

  if (params.postProcess) postProcessNode(doc, params.canvasWidth ?? 1200, pageId)
  await saveDocument(filePath, doc)

  const nodeIds = nodes.map((n) => n.id)
  const totalCount = flattenNodes(getDocChildren(doc, pageId)).length
  return {
    nodeIds,
    nodeCount: totalCount,
    postProcessed: params.postProcess || undefined,
  }
}
