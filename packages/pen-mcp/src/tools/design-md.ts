import { openDocument, resolveDocPath, saveDocument } from '../document-manager';
import {
  parseDesignMd,
  generateDesignMd,
  extractDesignMdFromDocument,
} from '../utils/design-md-parser';
import type { DesignMdSpec } from '@zseven-w/pen-types';

/**
 * design.md is now stored on the PenDocument (`doc.designMd`). It travels
 * with `.op`/`.pen` files and lives per-document. These handlers read
 * directly from the opened document and write via `saveDocument`.
 *
 * When `doc.designMd` is absent we auto-extract a spec from the document's
 * variables/typography for display purposes only — extraction is a
 * best-effort reverse inference, NOT persisted back unless the caller
 * explicitly sets it.
 */

export interface GetDesignMdParams {
  filePath?: string;
}

export interface SetDesignMdParams {
  filePath?: string;
  /** Raw markdown content of design.md */
  markdown?: string;
  /** If true, auto-extract from existing document content */
  autoExtract?: boolean;
}

export interface ExportDesignMdParams {
  filePath?: string;
}

function specHasContent(spec: DesignMdSpec): boolean {
  return !!(spec.colorPalette?.length || spec.typography?.fontFamily || spec.visualTheme);
}

/** Read the design.md spec from the document (falls back to extraction). */
export async function handleGetDesignMd(
  params: GetDesignMdParams,
): Promise<{ hasDesignMd: boolean; spec?: DesignMdSpec; markdown?: string }> {
  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);

  if (doc.designMd) {
    return {
      hasDesignMd: true,
      spec: doc.designMd,
      markdown: generateDesignMd(doc.designMd),
    };
  }

  // Fallback: auto-extract from document variables/typography. Not persisted.
  const extracted = extractDesignMdFromDocument(doc);
  if (specHasContent(extracted)) {
    return {
      hasDesignMd: true,
      spec: extracted,
      markdown: generateDesignMd(extracted),
    };
  }

  return { hasDesignMd: false };
}

/** Write design.md into the document (persisted via saveDocument). */
export async function handleSetDesignMd(
  params: SetDesignMdParams,
): Promise<{ success: boolean; spec?: DesignMdSpec }> {
  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);

  let spec: DesignMdSpec;
  if (params.autoExtract) {
    spec = extractDesignMdFromDocument(doc);
  } else if (params.markdown) {
    spec = parseDesignMd(params.markdown);
  } else {
    return { success: false };
  }

  doc.designMd = spec;
  await saveDocument(filePath, doc);

  return { success: true, spec };
}

/** Export design.md as markdown text (reads from doc.designMd first). */
export async function handleExportDesignMd(
  params: ExportDesignMdParams,
): Promise<{ markdown: string }> {
  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);

  if (doc.designMd) {
    return { markdown: generateDesignMd(doc.designMd) };
  }

  const spec = extractDesignMdFromDocument(doc);
  return { markdown: generateDesignMd(spec) };
}
