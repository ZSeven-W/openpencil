import type { ElementTree } from './helpers.js';

export interface ActivityRingParams {
  center_text: string;
  size?: number;
  thickness?: number;
}

/**
 * Apple-style progress ring with centered text. Emits the correct
 * pattern: frame(cornerRadius=size/2) + stroke + centered text —
 * never ellipse+sibling text (ellipse can't have children; sibling
 * stacks above/below instead of centering) and never layout=none +
 * nested frame (absolute positioning unreliable in Skia).
 *
 * Colorless by default per Style-Guide-orthogonal contract; caller
 * overrides via batch_design U-op.
 */
export function buildActivityRing(params: ActivityRingParams): ElementTree {
  const size = params.size ?? 80;
  const thickness = params.thickness ?? 8;
  return {
    type: 'frame',
    name: 'Activity Ring',
    role: 'activity-ring',
    width: size,
    height: size,
    cornerRadius: size / 2,
    fill: [],
    stroke: {
      thickness,
      fill: [{ type: 'solid', color: '#000000' }],
    },
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [
      {
        type: 'text',
        name: 'Center Text',
        role: 'heading',
        content: params.center_text,
        fontSize: 16,
        fontWeight: 700,
      },
    ],
  };
}
