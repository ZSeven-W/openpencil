/**
 * Build an SVG path `d` string for an ellipse arc (pie slice, donut segment, or ring).
 *
 * @param w         - Bounding box width
 * @param h         - Bounding box height
 * @param startDeg  - Start angle in degrees (0 = right / 3 o'clock, clockwise)
 * @param sweepDeg  - Sweep angle in degrees (extent of the arc)
 * @param inner     - Inner radius ratio 0..1 (0 = pie, >0 = donut)
 */
export function buildEllipseArcPath(
  w: number,
  h: number,
  startDeg: number,
  sweepDeg: number,
  inner: number,
): string {
  const startRad = (startDeg * Math.PI) / 180
  const sweepRad = (sweepDeg * Math.PI) / 180
  const endRad = startRad + sweepRad

  const rx = w / 2
  const ry = h / 2
  const cx = rx
  const cy = ry

  // Outer arc endpoints
  const ox1 = cx + rx * Math.cos(startRad)
  const oy1 = cy + ry * Math.sin(startRad)
  const ox2 = cx + rx * Math.cos(endRad)
  const oy2 = cy + ry * Math.sin(endRad)

  const large = sweepRad > Math.PI ? 1 : 0

  // Near-full circle (>=~359.9°): split into two semicircular arcs
  if (sweepRad > Math.PI * 2 - 0.02) {
    const midRad = startRad + Math.PI
    const omx = cx + rx * Math.cos(midRad)
    const omy = cy + ry * Math.sin(midRad)

    if (inner <= 0.001) {
      return [
        `M${f(ox1)} ${f(oy1)}`,
        `A${f(rx)} ${f(ry)} 0 1 1 ${f(omx)} ${f(omy)}`,
        `A${f(rx)} ${f(ry)} 0 1 1 ${f(ox1)} ${f(oy1)}`,
        'Z',
      ].join(' ')
    }

    const irx = rx * inner
    const iry = ry * inner
    const ix1 = cx + irx * Math.cos(startRad)
    const iy1 = cy + iry * Math.sin(startRad)
    const imx = cx + irx * Math.cos(midRad)
    const imy = cy + iry * Math.sin(midRad)
    return [
      `M${f(ox1)} ${f(oy1)}`,
      `A${f(rx)} ${f(ry)} 0 1 1 ${f(omx)} ${f(omy)}`,
      `A${f(rx)} ${f(ry)} 0 1 1 ${f(ox1)} ${f(oy1)}`,
      `L${f(ix1)} ${f(iy1)}`,
      `A${f(irx)} ${f(iry)} 0 1 0 ${f(imx)} ${f(imy)}`,
      `A${f(irx)} ${f(iry)} 0 1 0 ${f(ix1)} ${f(iy1)}`,
      'Z',
    ].join(' ')
  }

  if (inner <= 0.001) {
    // Pie slice: center → outer start → arc → close
    return `M${f(cx)} ${f(cy)} L${f(ox1)} ${f(oy1)} A${f(rx)} ${f(ry)} 0 ${large} 1 ${f(ox2)} ${f(oy2)} Z`
  }

  // Donut slice: outer arc → line to inner → inner arc (reversed) → close
  const irx = rx * inner
  const iry = ry * inner
  const ix1 = cx + irx * Math.cos(startRad)
  const iy1 = cy + iry * Math.sin(startRad)
  const ix2 = cx + irx * Math.cos(endRad)
  const iy2 = cy + iry * Math.sin(endRad)
  return [
    `M${f(ox1)} ${f(oy1)}`,
    `A${f(rx)} ${f(ry)} 0 ${large} 1 ${f(ox2)} ${f(oy2)}`,
    `L${f(ix2)} ${f(iy2)}`,
    `A${f(irx)} ${f(iry)} 0 ${large} 0 ${f(ix1)} ${f(iy1)}`,
    'Z',
  ].join(' ')
}

/** True when the arc parameters describe something other than a plain full ellipse. */
export function isArcEllipse(
  _startAngle?: number,
  sweepAngle?: number,
  innerRadius?: number,
): boolean {
  const sweep = sweepAngle ?? 360
  const inner = innerRadius ?? 0
  return sweep < 359.9 || inner > 0.001
}

function f(n: number): string {
  return Math.abs(n) < 0.005 ? '0' : parseFloat(n.toFixed(2)).toString()
}
