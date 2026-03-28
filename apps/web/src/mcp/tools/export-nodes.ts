import type { PenNode } from '@/types/pen'
import { openDocument, resolveDocPath } from '../document-manager'
import { findNodeInTree, getDocChildren } from '../utils/node-operations'

export interface ExportNodesParams {
  filePath?: string
  nodeIds?: string[]
  pageId?: string
}

interface ExportNodesResult {
  nodes: PenNode[]
  variables: Record<string, unknown>
  themes: unknown[]
}

export async function handleExportNodes(
  params: ExportNodesParams,
): Promise<ExportNodesResult> {
  const filePath = resolveDocPath(params.filePath)
  const doc = await openDocument(filePath)

  const pageChildren = getDocChildren(doc, params.pageId)

  let nodes: PenNode[]

  if (params.nodeIds && params.nodeIds.length > 0) {
    nodes = params.nodeIds
      .map((id) => findNodeInTree(pageChildren, id))
      .filter((n): n is PenNode => n !== undefined)
  } else {
    nodes = pageChildren
  }

  return {
    nodes,
    variables: doc.variables ?? {},
    themes: (doc as { themes?: unknown[] }).themes ?? [],
  }
}
