import { openDocument, resolveDocPath } from '../document-manager'
import { parseDesignMd, generateDesignMd, extractDesignMdFromDocument } from '../../utils/design-md-parser'
import type { DesignMdSpec } from '../../types/design-md'
import { setDesignMdForPrompt } from './design-prompt'

// In MCP context (stdio mode), there's no Zustand store.
// We keep a module-level cache for the design.md spec.
let _mcpDesignMd: DesignMdSpec | undefined

export interface GetDesignMdParams {
  filePath?: string
}

export interface SetDesignMdParams {
  filePath?: string
  /** Raw markdown content of design.md */
  markdown?: string
  /** If true, auto-extract from existing document content */
  autoExtract?: boolean
}

export interface ExportDesignMdParams {
  filePath?: string
}

/** Read the design.md spec. */
export async function handleGetDesignMd(
  params: GetDesignMdParams,
): Promise<{ hasDesignMd: boolean; spec?: DesignMdSpec; markdown?: string }> {
  // Try module cache first
  if (_mcpDesignMd) {
    return {
      hasDesignMd: true,
      spec: _mcpDesignMd,
      markdown: generateDesignMd(_mcpDesignMd),
    }
  }

  // Try to auto-extract from document
  const filePath = resolveDocPath(params.filePath)
  const doc = await openDocument(filePath)
  const spec = extractDesignMdFromDocument(doc)
  const hasContent = !!(spec.colorPalette?.length || spec.typography?.fontFamily || spec.visualTheme)

  if (hasContent) {
    _mcpDesignMd = spec
    return { hasDesignMd: true, spec, markdown: generateDesignMd(spec) }
  }

  return { hasDesignMd: false }
}

/** Import design.md content. */
export async function handleSetDesignMd(
  params: SetDesignMdParams,
): Promise<{ success: boolean; spec?: DesignMdSpec }> {
  let spec: DesignMdSpec

  if (params.autoExtract) {
    const filePath = resolveDocPath(params.filePath)
    const doc = await openDocument(filePath)
    spec = extractDesignMdFromDocument(doc)
  } else if (params.markdown) {
    spec = parseDesignMd(params.markdown)
  } else {
    return { success: false }
  }

  _mcpDesignMd = spec
  setDesignMdForPrompt(spec)

  return { success: true, spec }
}

/** Export design.md as markdown text. */
export async function handleExportDesignMd(
  params: ExportDesignMdParams,
): Promise<{ markdown: string }> {
  if (_mcpDesignMd) {
    return { markdown: generateDesignMd(_mcpDesignMd) }
  }

  // Auto-extract from document
  const filePath = resolveDocPath(params.filePath)
  const doc = await openDocument(filePath)
  const spec = extractDesignMdFromDocument(doc)
  return { markdown: generateDesignMd(spec) }
}
