import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface SpinnerV1Params {
  /** Outer diameter in px. Default 32. Clamped 16..128. */
  size?: number;
  /** Stroke thickness in px. Default 3. Clamped 1..16. */
  thickness?: number;
  /** Track color (the static ring). Default #E2E8F0 (slate-200). */
  track_color?: string;
  /** Active arc color. Default #2563EB (blue-600). */
  active_color?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_spinner_v0.
   * - `'dark'`: identical (track_color/active_color are caller-provided or
   *   iOS/Material defaults — these are UI state colors, not surface colors).
   * - `'system'`: identical.
   *
   * NOTE: track_color and active_color are caller-overridable parameters and
   * are treated as builder-private defaults (#E2E8F0 / #2563EB). They are
   * not tokenized across theme modes — use explicit track_color/active_color
   * overrides if you need theme-specific spinner colors.
   */
  theme?: V1Theme;
}

/**
 * Loading spinner (v1) — theme-aware variant of buildSpinner.
 * Light mode is byte-equal to add_spinner_v0.
 *
 * track_color and active_color are caller-controllable defaults.
 * Since no surface/text colors are hardcoded in the v0 builder body
 * (only the param defaults #E2E8F0 and #2563EB), all theme modes
 * emit the same tree — the caller provides explicit colors when needed.
 */
export function buildSpinnerV1(params: SpinnerV1Params): ElementTree {
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
