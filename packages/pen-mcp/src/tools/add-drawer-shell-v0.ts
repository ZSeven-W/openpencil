import { assignIdsRecursively, buildDrawerShell, type DrawerShellParams } from '@zseven-w/pen-core';
import type { handleBatchDesign } from './batch-design';
import { ensureParentExists, insertElementTree } from './element-tool-helpers';

export interface AddDrawerShellV0Params extends DrawerShellParams {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

export async function handleAddDrawerShellV0(
  params: AddDrawerShellV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const drawer = buildDrawerShell(params);
  assignIdsRecursively(drawer);
  return insertElementTree({ binding: 'drawerShell', tree: drawer, ...params });
}
