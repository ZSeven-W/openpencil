import { handleBatchDesign } from '@/mcp/tools/batch-design'
import { handleDesignSkeleton } from '@/mcp/tools/design-skeleton'
import { handleDesignContent } from '@/mcp/tools/design-content'
import { handleDesignRefine } from '@/mcp/tools/design-refine'
import { output, outputError, parseJsonArg, resolveArg } from '../output'

interface GlobalFlags {
  file?: string
  page?: string
}

export async function cmdDesign(
  args: string[],
  flags: GlobalFlags & { postProcess?: boolean; canvasWidth?: string },
): Promise<void> {
  const operations = await resolveArg(args[0])
  const result = await handleBatchDesign({
    filePath: flags.file,
    operations,
    postProcess: flags.postProcess !== false,
    canvasWidth: flags.canvasWidth ? parseInt(flags.canvasWidth, 10) : undefined,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdDesignSkeleton(
  args: string[],
  flags: GlobalFlags,
): Promise<void> {
  const json = (await parseJsonArg(args[0])) as Record<string, unknown>
  const result = await handleDesignSkeleton({
    filePath: flags.file,
    rootFrame: json.rootFrame as any,
    sections: json.sections as any,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdDesignContent(
  args: string[],
  flags: GlobalFlags & { canvasWidth?: string },
): Promise<void> {
  const sectionId = args[0]
  if (!sectionId) outputError('Usage: openpencil design:content <section-id> <json>')
  const json = (await parseJsonArg(args[1])) as Record<string, unknown>
  const result = await handleDesignContent({
    filePath: flags.file,
    sectionId,
    children: json.children as any,
    canvasWidth: flags.canvasWidth ? parseInt(flags.canvasWidth, 10) : undefined,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdDesignRefine(
  args: string[],
  flags: GlobalFlags & { rootId?: string; canvasWidth?: string },
): Promise<void> {
  if (!flags.rootId) outputError('Usage: openpencil design:refine --root-id <id>')
  const result = await handleDesignRefine({
    filePath: flags.file,
    rootId: flags.rootId!,
    canvasWidth: flags.canvasWidth ? parseInt(flags.canvasWidth, 10) : undefined,
    pageId: flags.page,
  })
  output(result)
}
