import { handleOpenDocument } from '@/mcp/tools/open-document'
import { handleBatchGet } from '@/mcp/tools/batch-get'
import { handleGetSelection } from '@/mcp/tools/get-selection'
import { openDocument, saveDocument, resolveDocPath } from '@/mcp/document-manager'
import { output, outputError } from '../output'

interface GlobalFlags {
  file?: string
  page?: string
}

export async function cmdOpen(args: string[], flags: GlobalFlags): Promise<void> {
  const result = await handleOpenDocument({ filePath: flags.file ?? args[0] })
  output(result)
}

export async function cmdSave(args: string[], flags: GlobalFlags): Promise<void> {
  const target = args[0]
  if (!target) outputError('Usage: openpencil save <file.op>')
  const doc = await openDocument(resolveDocPath(flags.file))
  await saveDocument(target, doc)
  output({ ok: true, filePath: target })
}

export async function cmdGet(args: string[], flags: GlobalFlags & {
  type?: string
  name?: string
  id?: string
  depth?: string
  parent?: string
}): Promise<void> {
  const patterns: { type?: string; name?: string }[] = []
  if (flags.type || flags.name) {
    patterns.push({ type: flags.type, name: flags.name })
  }

  const nodeIds: string[] = []
  if (flags.id) nodeIds.push(flags.id)

  const result = await handleBatchGet({
    filePath: flags.file,
    patterns: patterns.length ? patterns : undefined,
    nodeIds: nodeIds.length ? nodeIds : undefined,
    parentId: flags.parent,
    readDepth: flags.depth ? parseInt(flags.depth, 10) : undefined,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdSelection(flags: GlobalFlags & { depth?: string }): Promise<void> {
  const result = await handleGetSelection({
    filePath: flags.file,
    readDepth: flags.depth ? parseInt(flags.depth, 10) : undefined,
  })
  output(result)
}
