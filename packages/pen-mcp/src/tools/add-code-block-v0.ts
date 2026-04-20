import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddCodeBlockV0Params {
  code: string;
  language?: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Preformatted code block. Emits frame(fill_container, padding=[12,16],
 * cornerRadius=8, fill=gray-50) with one monospace text child that
 * preserves newlines in `code`. `language` is optional metadata that
 * becomes part of the frame name (e.g. "Code Block (typescript)") but
 * does not drive syntax highlighting — pen-core has no highlighting
 * pipeline; rendered as plain monospace.
 *
 * fontFamily is left unset because JetBrains Mono / Fira Code aren't
 * bundled with .op documents. The renderer falls back to the active
 * document's default font. Callers who need a mono face can inject
 * `fontFamily: 'JetBrains Mono'` via batch_design U-op.
 *
 * Text uses textGrowth='fixed-width' + width='fill_container' so long
 * lines wrap rather than overflow the container (see text-rules.md
 * "Text >15 chars MUST have textGrowth='fixed-width'").
 */
export async function handleAddCodeBlockV0(
  params: AddCodeBlockV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const nameSuffix = params.language ? ` (${params.language})` : '';
  const block = {
    type: 'frame',
    name: `Code Block${nameSuffix}`,
    role: 'code-block',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    padding: [12, 16],
    cornerRadius: 8,
    fill: [{ type: 'solid', color: '#F3F4F6' }],
    children: [
      {
        type: 'text',
        name: 'Code',
        role: 'code',
        content: params.code,
        fontSize: 13,
        fontWeight: 400,
        lineHeight: 1.5,
        width: 'fill_container',
        textGrowth: 'fixed-width',
      },
    ],
  };
  assignIdsRecursively(block);
  return insertElementTree({ binding: 'code_block', tree: block, ...params });
}
