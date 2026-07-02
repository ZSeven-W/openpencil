import type { ElementTree } from './helpers.js';

export interface SwitchParams {
  active?: boolean;
}

/**
 * iOS/Material toggle switch. Fixed 51×31 (iOS HIG), thumb 27×27
 * white. active=true → iOS green track + thumb pushed right via
 * justifyContent='flex-end'.
 */
export function buildSwitch(params: SwitchParams): ElementTree {
  const active = params.active === true;
  return {
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
}
