import { handleImportSvg } from '@zseven-w/pen-mcp';
import { saveDocument } from '@zseven-w/pen-mcp';
import { parseFigFile, figmaAllPagesToPenDocument } from '@zseven-w/pen-figma';
import { readFile } from 'node:fs/promises';
import { basename } from 'node:path';
import { output, outputError } from '../output';

interface GlobalFlags {
  file?: string;
  page?: string;
}

export async function cmdImportSvg(
  args: string[],
  flags: GlobalFlags & { parent?: string },
): Promise<void> {
  const svgPath = args[0];
  if (!svgPath) outputError('Usage: op import:svg <file.svg>');
  const result = await handleImportSvg({
    filePath: flags.file,
    svgPath,
    parent: flags.parent ?? null,
    pageId: flags.page,
  });
  output(result);
}

export async function cmdImportFigma(
  args: string[],
  flags: GlobalFlags & { out?: string },
): Promise<void> {
  const figPath = args[0];
  if (!figPath) outputError('Usage: op import:figma <file.fig> [--out output.op]');

  const buf = await readFile(figPath);
  const figFile = parseFigFile(buf.buffer as ArrayBuffer);
  const { document: doc, warnings } = figmaAllPagesToPenDocument(figFile, basename(figPath));

  const outPath = flags.out ?? figPath.replace(/\.fig$/, '.op');
  await saveDocument(outPath, doc);
  output({
    ok: true,
    filePath: outPath,
    pageCount: doc.pages?.length ?? 1,
    nodeCount: doc.pages
      ? doc.pages.reduce((s: number, p: { children: unknown[] }) => s + p.children.length, 0)
      : doc.children.length,
    warnings,
  });
}
