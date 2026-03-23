import {
  handleAddPage,
  handleRemovePage,
  handleRenamePage,
  handleReorderPage,
  handleDuplicatePage,
} from '@/mcp/tools/pages'
import { handleOpenDocument } from '@/mcp/tools/open-document'
import { output, outputError } from '../output'

interface GlobalFlags {
  file?: string
}

export async function cmdPageList(flags: GlobalFlags): Promise<void> {
  const result = await handleOpenDocument({ filePath: flags.file })
  output({
    pages: result.document.pages ?? [
      { id: 'default', name: 'Page 1', childCount: result.document.childCount },
    ],
  })
}

export async function cmdPageAdd(
  args: string[],
  flags: GlobalFlags & { name?: string },
): Promise<void> {
  const result = await handleAddPage({
    filePath: flags.file,
    name: flags.name ?? args[0],
  })
  output(result)
}

export async function cmdPageRemove(
  args: string[],
  flags: GlobalFlags,
): Promise<void> {
  const pageId = args[0]
  if (!pageId) outputError('Usage: op page remove <page-id>')
  const result = await handleRemovePage({
    filePath: flags.file,
    pageId,
  })
  output(result)
}

export async function cmdPageRename(
  args: string[],
  flags: GlobalFlags,
): Promise<void> {
  const [pageId, name] = args
  if (!pageId || !name) outputError('Usage: op page rename <page-id> <name>')
  const result = await handleRenamePage({
    filePath: flags.file,
    pageId,
    name,
  })
  output(result)
}

export async function cmdPageReorder(
  args: string[],
  flags: GlobalFlags,
): Promise<void> {
  const [pageId, indexStr] = args
  if (!pageId || !indexStr) outputError('Usage: op page reorder <page-id> <index>')
  const result = await handleReorderPage({
    filePath: flags.file,
    pageId,
    index: parseInt(indexStr, 10),
  })
  output(result)
}

export async function cmdPageDuplicate(
  args: string[],
  flags: GlobalFlags,
): Promise<void> {
  const pageId = args[0]
  if (!pageId) outputError('Usage: op page duplicate <page-id>')
  const result = await handleDuplicatePage({
    filePath: flags.file,
    pageId,
  })
  output(result)
}
