/**
 * Build SVG 路径 `d` 椭圆弧（饼图、圆环段或环）的字符串。
 *
 * @param w         - Bounding 框宽度
 * @param h         - Bounding 盒子高度
 * @param startDeg  - Start 角度（0 = 右/3 点钟，顺时针）
 * @param sweepDeg  - Sweep 角度（弧的长度）
 * @param inner     - Inner 半径比 0..1（0 = 饼图，>0 = 甜甜圈）
 */
export function buildEllipseArcPath(
  w: number,
  h: number,
  startDeg: number,
  sweepDeg: number,
  inner: number,
): string {
  const startRad = (startDeg * Math.PI) / 180;
  const sweepRad = (sweepDeg * Math.PI) / 180;
  const endRad = startRad + sweepRad;

  const rx = w / 2;
  const ry = h / 2;
  const cx = rx;
  const cy = ry;

  // Outer 圆弧端点
  const ox1 = cx + rx * Math.cos(startRad);
  const oy1 = cy + ry * Math.sin(startRad);
  const ox2 = cx + rx * Math.cos(endRad);
  const oy2 = cy + ry * Math.sin(endRad);

  const large = sweepRad > Math.PI ? 1 : 0;

  // Near-整圆（>=~359.9°）：分成两个半圆弧
  if (sweepRad > Math.PI * 2 - 0.02) {
    const midRad = startRad + Math.PI;
    const omx = cx + rx * Math.cos(midRad);
    const omy = cy + ry * Math.sin(midRad);

    if (inner <= 0.001) {
      return [
        `M${f(ox1)} ${f(oy1)}`,
        `A${f(rx)} ${f(ry)} 0 1 1 ${f(omx)} ${f(omy)}`,
        `A${f(rx)} ${f(ry)} 0 1 1 ${f(ox1)} ${f(oy1)}`,
        'Z',
      ].join(' ');
    }

    const irx = rx * inner;
    const iry = ry * inner;
    const ix1 = cx + irx * Math.cos(startRad);
    const iy1 = cy + iry * Math.sin(startRad);
    const imx = cx + irx * Math.cos(midRad);
    const imy = cy + iry * Math.sin(midRad);
    return [
      `M${f(ox1)} ${f(oy1)}`,
      `A${f(rx)} ${f(ry)} 0 1 1 ${f(omx)} ${f(omy)}`,
      `A${f(rx)} ${f(ry)} 0 1 1 ${f(ox1)} ${f(oy1)}`,
      `L${f(ix1)} ${f(iy1)}`,
      `A${f(irx)} ${f(iry)} 0 1 0 ${f(imx)} ${f(imy)}`,
      `A${f(irx)} ${f(iry)} 0 1 0 ${f(ix1)} ${f(iy1)}`,
      'Z',
    ].join(' ');
  }

  if (inner <= 0.001) {
    // Pie slice: center → outer start → arc → close
    return `M${f(cx)} ${f(cy)} L${f(ox1)} ${f(oy1)} A${f(rx)} ${f(ry)} 0 ${large} 1 ${f(ox2)} ${f(oy2)} Z`;
  }

  // Donut slice: outer arc → line to inner → inner arc (reversed) → close
  const irx = rx * inner;
  const iry = ry * inner;
  const ix1 = cx + irx * Math.cos(startRad);
  const iy1 = cy + iry * Math.sin(startRad);
  const ix2 = cx + irx * Math.cos(endRad);
  const iy2 = cy + iry * Math.sin(endRad);
  return [
    `M${f(ox1)} ${f(oy1)}`,
    `A${f(rx)} ${f(ry)} 0 ${large} 1 ${f(ox2)} ${f(oy2)}`,
    `L${f(ix2)} ${f(iy2)}`,
    `A${f(irx)} ${f(iry)} 0 ${large} 0 ${f(ix1)} ${f(iy1)}`,
    'Z',
  ].join(' ');
}

/** True when the arc parameters describe something other than a plain full ellipse. */
export function isArcEllipse(
  _startAngle?: number,
  sweepAngle?: number,
  innerRadius?: number,
): boolean {
  const sweep = sweepAngle ?? 360;
  const inner = innerRadius ?? 0;
  return sweep < 359.9 || inner > 0.001;
}

function f(n: number): string {
  return Math.abs(n) < 0.005 ? '0' : parseFloat(n.toFixed(2)).toString();
}
