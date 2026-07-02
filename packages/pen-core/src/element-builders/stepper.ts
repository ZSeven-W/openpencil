import type { ElementTree } from './helpers.js';

export interface StepperParams {
  total: number;
  current?: number;
}

/**
 * Horizontal numbered stepper: (1)───(2)───(3). Connectors are
 * rectangle(fill_container, h=2) so pen-core splits space equally
 * between adjacent circles. Done/current circles primary; pending
 * gray. Connectors before (or at) current are primary; after gray.
 */
export function buildStepper(params: StepperParams): ElementTree {
  const total = Math.max(1, Math.floor(params.total));
  const current = Math.max(0, Math.min(total - 1, Math.floor(params.current ?? 0)));
  const children: ElementTree[] = [];
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
          fill: [{ type: 'solid', color: done ? '#FFFFFF' : '#6B7280' }],
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
  return {
    type: 'frame',
    name: 'Stepper',
    role: 'stepper',
    width: 'fill_container',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 0,
    children,
  };
}
