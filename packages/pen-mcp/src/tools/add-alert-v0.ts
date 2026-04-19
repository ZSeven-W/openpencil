import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddAlertV0Params {
  message: string;
  icon?: string;
  dismissible?: boolean;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Inline alert / callout banner. One-row layout: [icon?] + message +
 * [close-x?]. width=fill_container. Colors stay neutral by default — caller
 * applies semantic color (info/success/warning/error) via a follow-up
 * batch_design U-op.
 *
 * role='alert'.
 */
export async function handleAddAlertV0(
  params: AddAlertV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const children: Record<string, unknown>[] = [];
  if (params.icon) {
    children.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  children.push({
    type: 'text',
    name: 'Message',
    role: 'alert-message',
    content: params.message,
    fontSize: 14,
    fontWeight: 400,
  });
  if (params.dismissible) {
    children.push({
      type: 'icon_font',
      name: 'Close',
      role: 'alert-close',
      iconFontName: 'x',
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  const alert = {
    type: 'frame',
    name: 'Alert',
    role: 'alert',
    width: 'fill_container',
    cornerRadius: 8,
    padding: [12, 16],
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    children,
  };
  assignIdsRecursively(alert);
  return insertElementTree({ binding: 'alert', tree: alert, ...params });
}
