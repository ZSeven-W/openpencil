import {
  openDocument,
  createEmptyDocument,
  saveDocument,
  fileExists,
  resolveDocPath,
  LIVE_CANVAS_PATH,
} from '../document-manager'
import { flattenNodes, getDocChildren } from '../utils/node-operations'
import { buildDesignPrompt } from './design-prompt'
import type { PenDocument, PenNode } from '../../types/pen'

export interface OpenDocumentParams {
  filePath?: string
}

export interface OpenDocumentResult {
  filePath: string
  document: {
    version: string
    name?: string
    childCount: number
    pageCount: number
    pages?: { id: string; name: string; childCount: number }[]
    hasVariables: boolean
    hasThemes: boolean
  }
  context: string
  designPrompt: string
}

export async function handleOpenDocument(
  params: OpenDocumentParams,
): Promise<OpenDocumentResult> {
  let filePath: string
  let doc: PenDocument

  if (!params.filePath || params.filePath === LIVE_CANVAS_PATH) {
    // Live canvas mode: connect to the running Electron/dev server.
    // openDocument() now returns a more precise diagnostic when sync is unavailable.
    filePath = LIVE_CANVAS_PATH
    doc = await openDocument(LIVE_CANVAS_PATH)
  } else {
    filePath = resolveDocPath(params.filePath)
    const exists = await fileExists(filePath)
    if (exists) {
      doc = await openDocument(filePath)
    } else {
      // Create new file at specified path
      doc = createEmptyDocument()
      await saveDocument(filePath, doc)
    }
  }

  const pages = doc.pages?.map((p) => ({
    id: p.id,
    name: p.name,
    childCount: p.children.length,
  }))
  const totalChildren = doc.pages
    ? doc.pages.reduce((sum, p) => sum + p.children.length, 0)
    : doc.children.length

  // Only include full design prompt when document is empty or has only empty frames.
  // For non-empty documents, include a brief note to reduce context window waste.
  const children = getDocChildren(doc)
  const isEmpty =
    totalChildren === 0 ||
    (children.length === 1 &&
      children[0].type === 'frame' &&
      (!('children' in children[0]) || !(children[0] as any).children?.length))

  return {
    filePath,
    document: {
      version: doc.version,
      name: doc.name,
      childCount: totalChildren,
      pageCount: doc.pages?.length ?? 1,
      pages,
      hasVariables: !!doc.variables && Object.keys(doc.variables).length > 0,
      hasThemes: !!doc.themes && Object.keys(doc.themes).length > 0,
    },
    context: buildDocumentContext(doc),
    designPrompt: isEmpty
      ? buildDesignPrompt()
      : 'Document has existing content. Use batch_design or insert_node with postProcess=true to add/modify designs. ' +
        'For complex multi-section designs, use the layered workflow: design_skeleton → design_content (per section) → design_refine. ' +
        'Call get_design_prompt(section="planning") for layered workflow guide, or get_design_prompt() for full guidelines.',
  }
}

// ---------------------------------------------------------------------------
// Document context builder (merged from document-context.ts)
// ---------------------------------------------------------------------------

function buildDocumentContext(doc: PenDocument): string {
  const children = getDocChildren(doc)
  const allNodes = flattenNodes(children)

  if (allNodes.length === 0) {
    return 'Empty document. No existing nodes.'
  }

  const nodeSummary = allNodes
    .slice(0, 20)
    .map((n) => `${n.type}:${n.name ?? n.id}`)
    .join(', ')

  const canvasSize = estimateCanvasSize(children)

  const parts: string[] = []
  parts.push(`DOCUMENT SUMMARY:`)
  parts.push(`- Total nodes: ${allNodes.length}`)
  parts.push(`- Canvas size: ${canvasSize.width}x${canvasSize.height}`)
  if (nodeSummary) {
    parts.push(`- Nodes (first 20): ${nodeSummary}`)
  }

  // Hint about empty root frames that will be auto-replaced
  const emptyRootFrames = children.filter(
    (n) =>
      n.type === 'frame' &&
      (!('children' in n) || !(n as any).children || (n as any).children.length === 0),
  )
  if (emptyRootFrames.length > 0) {
    const info = emptyRootFrames
      .map((f) => {
        const w = typeof (f as any).width === 'number' ? (f as any).width : 1200
        const h = typeof (f as any).height === 'number' ? (f as any).height : 800
        return `"${f.name ?? f.id}" (id: ${f.id}, ${w}x${h})`
      })
      .join(', ')
    parts.push(
      `- Empty root frame(s): ${info} — will be auto-replaced when you insert a root-level frame via I(null, ...)`,
    )
  }

  const pageLines = buildPageContext(doc)
  if (pageLines) parts.push('', pageLines)

  const variableLines = buildVariableContext(doc)
  if (variableLines) parts.push('', variableLines)

  const themeLines = buildThemeContext(doc)
  if (themeLines) parts.push('', themeLines)

  return parts.join('\n')
}

function estimateCanvasSize(children: PenNode[]): {
  width: number
  height: number
} {
  for (const node of children) {
    if (
      node.type === 'frame' &&
      typeof node.width === 'number' &&
      typeof node.height === 'number'
    ) {
      if (node.width <= 500 && node.height >= 700) {
        return { width: 375, height: 812 }
      }
      return { width: node.width, height: node.height }
    }
  }
  return { width: 1200, height: 800 }
}

function buildVariableContext(doc: PenDocument): string {
  if (!doc.variables || Object.keys(doc.variables).length === 0) return ''

  const lines = ['DOCUMENT VARIABLES (use "$name" to reference):']
  for (const [name, def] of Object.entries(doc.variables)) {
    const themed = Array.isArray(def.value) ? ' [themed]' : ''
    const displayValue = Array.isArray(def.value)
      ? String(def.value[0]?.value ?? '')
      : String(def.value)
    lines.push(`  - ${name} (${def.type}): ${displayValue}${themed}`)
  }
  return lines.join('\n')
}

function buildThemeContext(doc: PenDocument): string {
  if (!doc.themes || Object.keys(doc.themes).length === 0) return ''

  const entries = Object.entries(doc.themes)
    .map(([axis, variants]) => `${axis}: [${variants.join(',')}]`)
    .join('; ')
  return `Themes: ${entries}`
}

function buildPageContext(doc: PenDocument): string {
  if (!doc.pages || doc.pages.length <= 1) return ''

  const pageList = doc.pages
    .map((p) => `${p.name} (${p.children.length} nodes)`)
    .join(', ')
  return `Pages: ${pageList}`
}
