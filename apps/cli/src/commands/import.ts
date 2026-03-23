import { handleImportSvg } from '@/mcp/tools/import-svg'
import { openDocument, saveDocument, resolveDocPath } from '@/mcp/document-manager'
import { parseFigFile, figmaAllPagesToPenDocument } from '@zseven-w/pen-figma'
import { readFile } from 'node:fs/promises'
import { output, outputError } from '../output'

interface GlobalFlags {
  file?: string
  page?: string
}

export async function cmdImportSvg(
  args: string[],
  flags: GlobalFlags & { parent?: string },
): Promise<void> {
  const svgPath = args[0]
  if (!svgPath) outputError('Usage: op import:svg <file.svg>')
  const result = await handleImportSvg({
    filePath: flags.file,
    svgPath,
    parent: flags.parent ?? null,
    pageId: flags.page,
  })
  output(result)
}

export async function cmdImportFigma(
  args: string[],
  flags: GlobalFlags & { out?: string },
): Promise<void> {
  const figPath = args[0]
  if (!figPath) outputError('Usage: op import:figma <file.fig> [--out output.op]')

  const buf = await readFile(figPath)
  const figFile = parseFigFile(new Uint8Array(buf))
  const doc = figmaAllPagesToPenDocument(figFile)

  const outPath = flags.out ?? figPath.replace(/\.fig$/, '.op')
  await saveDocument(outPath, doc)
  output({
    ok: true,
    filePath: outPath,
    pageCount: doc.pages?.length ?? 1,
    nodeCount: doc.pages
      ? doc.pages.reduce((s, p) => s + p.children.length, 0)
      : doc.children.length,
  })
}
