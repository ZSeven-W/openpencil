import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddProgressBarV0Params {
  value?: number;
  bar_width?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Linear progress bar. Fixed-pixel width (default 240) so the progress
 * fill can be computed as a deterministic sub-width (value/100 * bar_width).
 * pen-core has no flex-basis/percent sizing for child widths, so a pixel
 * width is the honest representation. Caller resizes via batch_design
 * U-op if they need a responsive bar.
 *
 * roles: 'progress-bar' (track) / 'progress-bar-fill' (inner).
 */
export async function handleAddProgressBarV0(
  params: AddProgressBarV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const raw = params.value ?? 50;
  const value = Math.max(0, Math.min(100, raw));
  const barWidth = params.bar_width ?? 240;
  const fillWidth = Math.max(0, Math.round((barWidth * value) / 100));
  const children: Record<string, unknown>[] = [];
  if (fillWidth > 0) {
    children.push({
      type: 'rectangle',
      name: 'Fill',
      role: 'progress-bar-fill',
      width: fillWidth,
      height: 8,
      cornerRadius: 4,
      fill: [{ type: 'solid', color: '#2563EB' }],
    });
  }
  const track = {
    type: 'frame',
    name: 'Progress Bar',
    role: 'progress-bar',
    width: barWidth,
    height: 8,
    cornerRadius: 4,
    fill: [{ type: 'solid', color: '#E5E7EB' }],
    layout: 'horizontal',
    alignItems: 'center',
    children,
  };
  assignIdsRecursively(track);
  return insertElementTree({ binding: 'progress', tree: track, ...params });
}
