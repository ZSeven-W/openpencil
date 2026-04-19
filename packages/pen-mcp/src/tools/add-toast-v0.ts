import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddToastV0Params {
  message: string;
  icon?: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Floating pill notification. Dark fill, cornerRadius=24 (pill shape),
 * content-sized so it doesn't stretch across the canvas. Caller positions
 * it at the bottom/top of a screen via parent_id or later batch_design U-op.
 *
 * role='toast'.
 */
export async function handleAddToastV0(
  params: AddToastV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const children: Record<string, unknown>[] = [];
  if (params.icon) {
    children.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 18,
      height: 18,
    });
  }
  children.push({
    type: 'text',
    name: 'Message',
    role: 'toast-message',
    content: params.message,
    fontSize: 14,
    fontWeight: 500,
  });
  const toast = {
    type: 'frame',
    name: 'Toast',
    role: 'toast',
    width: 'fit_content',
    cornerRadius: 24,
    padding: [12, 20],
    fill: [{ type: 'solid', color: '#111827' }],
    layout: 'horizontal',
    alignItems: 'center',
    gap: 8,
    children,
  };
  assignIdsRecursively(toast);
  return insertElementTree({ binding: 'toast', tree: toast, ...params });
}
