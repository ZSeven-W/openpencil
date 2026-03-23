import {
  handleInsertNode,
  handleUpdateNode,
  handleDeleteNode,
  handleMoveNode,
  handleCopyNode,
  handleReplaceNode,
} from '@/mcp/tools/node-crud'
import { output, outputError, parseJsonArg } from '../output'

interface GlobalFlags {
  file?: string
  page?: string
}

export async function cmdInsert(
  args: string[],
  flags: GlobalFlags & { parent?: string; index?: string; postProcess?: boolean },
): Promise<void> {
  const data = (await parseJsonArg(args[0])) as Record<string, unknown>
  const result = await handleInsertNode({
    filePath: flags.file,
    parent: flags.parent ?? null,
    data,
    postProcess: flags.postProcess,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdUpdate(
  args: string[],
  flags: GlobalFlags & { postProcess?: boolean },
): Promise<void> {
  const nodeId = args[0]
  if (!nodeId) outputError('Usage: openpencil update <node-id> <json>')
  const data = (await parseJsonArg(args[1])) as Record<string, unknown>
  const result = await handleUpdateNode({
    filePath: flags.file,
    nodeId,
    data,
    postProcess: flags.postProcess,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdDelete(
  args: string[],
  flags: GlobalFlags,
): Promise<void> {
  const nodeId = args[0]
  if (!nodeId) outputError('Usage: openpencil delete <node-id>')
  const result = await handleDeleteNode({
    filePath: flags.file,
    nodeId,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdMove(
  args: string[],
  flags: GlobalFlags & { parent?: string; index?: string },
): Promise<void> {
  const nodeId = args[0]
  if (!nodeId) outputError('Usage: openpencil move <node-id> --parent <parent-id>')
  const result = await handleMoveNode({
    filePath: flags.file,
    nodeId,
    parent: flags.parent ?? null,
    index: flags.index ? parseInt(flags.index, 10) : undefined,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdCopy(
  args: string[],
  flags: GlobalFlags & { parent?: string },
): Promise<void> {
  const sourceId = args[0]
  if (!sourceId) outputError('Usage: openpencil copy <source-id> [--parent <parent-id>]')
  const result = await handleCopyNode({
    filePath: flags.file,
    sourceId,
    parent: flags.parent ?? null,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdReplace(
  args: string[],
  flags: GlobalFlags & { postProcess?: boolean },
): Promise<void> {
  const nodeId = args[0]
  if (!nodeId) outputError('Usage: openpencil replace <node-id> <json>')
  const data = (await parseJsonArg(args[1])) as Record<string, unknown>
  const result = await handleReplaceNode({
    filePath: flags.file,
    nodeId,
    data,
    postProcess: flags.postProcess,
    pageId: flags.page,
  })
  output(result)
}
