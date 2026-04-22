import type { ElementTree } from './helpers.js';

export interface SpinnerParams {
  /** Outer diameter in px. Default 32. Clamped 16..128. */
  size?: number;
  /** Stroke thickness in px. Default 3. Clamped 1..16. */
  thickness?: number;
  /** Track color (the static ring). Default #E2E8F0 (slate-200). */
  track_color?: string;
  /** Active arc color. Default #2563EB (blue-600). */
  active_color?: string;
}

/**
 * Loading spinner — a ring with a 3/4 sweep active arc. Static
 * (no animation; pen-core is still-frame only). Appearance matches
 * typical Material/iOS spinner mid-animation.
 *
 * Structure (two stacked ellipse nodes at same origin):
 *   frame(size², layout=none)
 *     ├ ellipse(size², track_color stroke only, role='spinner-track')
 *     └ ellipse(size², active_color stroke, sweepAngle=270, role='spinner-arc')
 *
 * NOT the "stacked ellipses for a ring" anti-pattern: these ellipses
 * have DIFFERENT sweep ranges — track is a full ring (implicit 360),
 * active is a 270° arc. rewriteLlmAntiPatterns only rewrites when
 * both ellipses are full-sweep duplicates.
 */
export function buildSpinner(params: SpinnerParams): ElementTree {
  const size = Math.max(16, Math.min(128, Math.floor(params.size ?? 32)));
  const thickness = Math.max(1, Math.min(16, Math.floor(params.thickness ?? 3)));
  const trackColor = params.track_color ?? '#E2E8F0';
  const activeColor = params.active_color ?? '#2563EB';

  return {
    type: 'frame',
    name: 'Spinner',
    role: 'spinner',
    width: size,
    height: size,
    layout: 'none',
    children: [
      {
        type: 'ellipse',
        name: 'Track',
        role: 'spinner-track',
        x: 0,
        y: 0,
        width: size,
        height: size,
        fill: [],
        stroke: { thickness, fill: [{ type: 'solid', color: trackColor }] },
      },
      {
        type: 'ellipse',
        name: 'Active Arc',
        role: 'spinner-arc',
        x: 0,
        y: 0,
        width: size,
        height: size,
        startAngle: -90,
        sweepAngle: 270,
        fill: [],
        stroke: { thickness, fill: [{ type: 'solid', color: activeColor }] },
      },
    ],
  };
}
