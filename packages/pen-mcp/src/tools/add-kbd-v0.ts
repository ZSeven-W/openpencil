import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddKbdV0Params {
  keys: string[];
  separator?: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Keyboard shortcut badge ("⌘ K" / "Ctrl + Shift + P"). Each entry in
 * `keys` becomes a small bordered box with the key glyph; entries are
 * joined with a `separator` glyph between them (default: "+" for
 * PC-style, but common values are "" for Mac-symbol runs and " " for
 * single-key).
 *
 * Each key cell is frame(padding=[2,6], cornerRadius=4, role='kbd-key')
 * with a 1px neutral stroke — no Style-Guide color baked in. Monospace
 * font is omitted so the renderer's default font carries (bundling a
 * monospace face per document is out of scope for v0).
 */
export async function handleAddKbdV0(
  params: AddKbdV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const keys = params.keys.filter((k): k is string => typeof k === 'string' && k.length > 0);
  if (keys.length === 0) {
    throw new Error('add_kbd_v0 requires at least one non-empty key in `keys`.');
  }
  const separator = params.separator ?? '+';
  const children: Record<string, unknown>[] = [];
  keys.forEach((key, idx) => {
    children.push({
      type: 'frame',
      name: `Key ${idx + 1}`,
      role: 'kbd-key',
      width: 'fit_content',
      height: 'fit_content',
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      padding: [2, 6],
      cornerRadius: 4,
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#D1D5DB' }] },
      fill: [{ type: 'solid', color: '#F3F4F6' }],
      children: [
        {
          type: 'text',
          name: 'Glyph',
          role: 'kbd-glyph',
          content: key,
          fontSize: 12,
          fontWeight: 500,
        },
      ],
    });
    if (idx < keys.length - 1 && separator.length > 0) {
      children.push({
        type: 'text',
        name: 'Separator',
        role: 'kbd-separator',
        content: separator,
        fontSize: 12,
        fontWeight: 400,
      });
    }
  });
  const kbd = {
    type: 'frame',
    name: 'Keyboard Shortcut',
    role: 'kbd',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 4,
    children,
  };
  assignIdsRecursively(kbd);
  return insertElementTree({ binding: 'kbd', tree: kbd, ...params });
}
