import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddSwitchV0Params {
  active?: boolean;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * iOS/Material toggle switch. Fixed 51×31 (iOS HIG), thumb 27×27 white.
 * active=true → track uses iOS green + thumb pushed right via
 * justifyContent='flex-end'. Off is the default.
 *
 * role='switch'. Style later via batch_design U-op if theme tokens needed.
 */
export async function handleAddSwitchV0(
  params: AddSwitchV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const active = params.active === true;
  const track = {
    type: 'frame',
    name: active ? 'Switch (on)' : 'Switch (off)',
    role: 'switch',
    width: 51,
    height: 31,
    cornerRadius: 16,
    fill: [{ type: 'solid', color: active ? '#34C759' : '#E5E5EA' }],
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: active ? 'flex-end' : 'flex-start',
    padding: [2],
    children: [
      {
        type: 'frame',
        name: 'Thumb',
        role: 'switch-thumb',
        width: 27,
        height: 27,
        cornerRadius: 14,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
      },
    ],
  };
  assignIdsRecursively(track);
  return insertElementTree({ binding: 'switch', tree: track, ...params });
}
