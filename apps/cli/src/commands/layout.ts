import { handleSnapshotLayout } from '@/mcp/tools/snapshot-layout'
import { handleFindEmptySpace } from '@/mcp/tools/find-empty-space'
import { output, outputError } from '../output'

interface GlobalFlags {
  file?: string
  page?: string
}

export async function cmdLayout(
  flags: GlobalFlags & { parent?: string; depth?: string },
): Promise<void> {
  const result = await handleSnapshotLayout({
    filePath: flags.file,
    parentId: flags.parent,
    maxDepth: flags.depth ? parseInt(flags.depth, 10) : undefined,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdFindSpace(
  flags: GlobalFlags & { direction?: string; width?: string; height?: string },
): Promise<void> {
  const result = await handleFindEmptySpace({
    filePath: flags.file,
    direction: (flags.direction as 'right' | 'bottom' | 'left' | 'top') ?? 'right',
    width: flags.width ? parseInt(flags.width, 10) : undefined,
    height: flags.height ? parseInt(flags.height, 10) : undefined,
    pageId: flags.page,
  })
  output(result)
}
