import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddStepperV0Params {
  total: number;
  current?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Horizontal numbered stepper: (1)───(2)───(3). Connectors are
 * `rectangle(fill_container, h=2)` so they fill the space between
 * adjacent circles — pen-core splits multiple fill_container siblings
 * equally. Circles are 24×24 with the step index displayed.
 *
 * Done / current circles use primary fill; pending circles use gray.
 * Connectors before (or at) the current step are primary; after are gray.
 *
 * roles: 'stepper' / 'step' | 'step-active' / 'step-connector' |
 * 'step-connector-active'.
 */
export async function handleAddStepperV0(
  params: AddStepperV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const total = Math.max(1, Math.floor(params.total));
  const current = Math.max(0, Math.min(total - 1, Math.floor(params.current ?? 0)));
  const children: Record<string, unknown>[] = [];
  for (let i = 0; i < total; i += 1) {
    const done = i <= current;
    children.push({
      type: 'frame',
      name: `Step ${i + 1}`,
      role: done ? 'step-active' : 'step',
      width: 24,
      height: 24,
      cornerRadius: 12,
      fill: [{ type: 'solid', color: done ? '#2563EB' : '#E5E7EB' }],
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'text',
          name: 'Number',
          content: String(i + 1),
          fontSize: 13,
          fontWeight: 600,
        },
      ],
    });
    if (i < total - 1) {
      const doneConnector = i < current;
      children.push({
        type: 'rectangle',
        name: `Connector ${i}`,
        role: doneConnector ? 'step-connector-active' : 'step-connector',
        width: 'fill_container',
        height: 2,
        fill: [{ type: 'solid', color: doneConnector ? '#2563EB' : '#E5E7EB' }],
      });
    }
  }
  const stepper = {
    type: 'frame',
    name: 'Stepper',
    role: 'stepper',
    width: 'fill_container',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 0,
    children,
  };
  assignIdsRecursively(stepper);
  return insertElementTree({ binding: 'stepper', tree: stepper, ...params });
}
