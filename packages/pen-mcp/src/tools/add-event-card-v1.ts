import { assignIdsRecursively, buildEventCardV1, type EventCardV1Params } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddEventCardV1Params extends EventCardV1Params {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Calendar event card (v1) — theme-aware variant of add_event_card_v0.
 * Supports 'light' (v0 byte-parity), 'dark', and 'system' theme modes.
 * Card bg → surface, card stroke → border, date column → surface2,
 * title/day → textPrimary, meta → textMuted. Accent + white-on-accent
 * are caller-supplied brand colors, passed through in all modes.
 */
export async function handleAddEventCardV1(
  params: AddEventCardV1Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const tree = buildEventCardV1(params);
  assignIdsRecursively(tree);
  return insertElementTree({ binding: 'ec', tree, ...params });
}
