import type { CanvasKit, Path } from 'canvaskit-wasm';

/**
 * Normalize
 * SVG CanvasKit 解析器的路径数据： - Add 命令字母和数字之间的空格 -
 * Handle 负号数字分隔符（例如“10-5”->“10 -5”） - Normalize
 * 逗号分隔符到空格 - Separate 连接弧标志（例如“a2 2”） 0 012 2" -> "a2 2 0 0 1 2 2")
 *
 */
export function sanitizeSvgPath(d: string): string {
  let result = d
    // Add 命令字母和后面的 number/sign 之间有空格
    .replace(/([MLCQZAHVSmlcqzahvsTt])([0-9.+-])/g, '$1 $2')
    // Add 数字和后面的负号之间的空格（数字分隔符）
    .replace(/(\d)-/g, '$1 -')
    // Replace 逗号加空格
    .replace(/,/g, ' ')
    // Collapse 多个空格
    .replace(/\s+/g, ' ')
    .trim();

  // Separate 连接弧标志：在 SVG arc 命令中，大弧和扫描标志是单个数字（0 或 1），可以相互连接并与以下数字连接。例如“a2 2
  // 0 012 2”->“a2 2 0 0 1 2 2”
  result = result.replace(
    /([aA])\s*([\d.e+-]+)\s+([\d.e+-]+)\s+([\d.e+-]+)\s+([01])([01])([\d.+-])/g,
    '$1 $2 $3 $4 $5 $6 $7',
  );
  // Handle 所有三个（旋转+标志）不带空格连接的情况，例如"a4 4 0100-8" 其中 0100 = 旋转 = 0，大弧 = 1，扫描 = 0，则 0 是 x
  // 的开始
  result = result.replace(
    /([aA])\s*([\d.e+-]+)\s+([\d.e+-]+)\s+(\d)([01])([01])([\d.+-])/g,
    '$1 $2 $3 $4 $5 $6 $7',
  );

  return result;
}

/** 如果路径字符串包含 NaN 或 Infinity 值，则 Returns 为 true。 */
export function hasInvalidNumbers(d: string): boolean {
  return /NaN|Infinity/i.test(d);
}

/**
 * Convert 将
 * SVG 圆弧段转换为三次贝塞尔曲线并将其添加到路径中。 Based 关于 W3C SVG 弧到立方转换的实现说明。
 */
function arcToCubics(
  path: Path,
  x1: number,
  y1: number,
  rxIn: number,
  ryIn: number,
  largeArc: boolean,
  sweep: boolean,
  x2: number,
  y2: number,
): void {
  // Degenerate：开始==结束
  if (x1 === x2 && y1 === y2) return;

  let rx = Math.abs(rxIn);
  let ry = Math.abs(ryIn);

  const dx = (x1 - x2) / 2;
  const dy = (y1 - y2) / 2;
  // Simplified：忽略旋转（大多数图标使用 rotation=0）
  const x1p = dx;
  const y1p = dy;

  // Correct 半径
  let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
  if (lambda > 1) {
    const s = Math.sqrt(lambda);
    rx *= s;
    ry *= s;
  }

  const rxSq = rx * rx;
  const rySq = ry * ry;
  const x1pSq = x1p * x1p;
  const y1pSq = y1p * y1p;

  let sq = (rxSq * rySq - rxSq * y1pSq - rySq * x1pSq) / (rxSq * y1pSq + rySq * x1pSq);
  if (sq < 0) sq = 0;
  let root = Math.sqrt(sq);
  if (largeArc === sweep) root = -root;

  const cxp = (root * rx * y1p) / ry;
  const cyp = (-root * ry * x1p) / rx;

  const cx = cxp + (x1 + x2) / 2;
  const cy = cyp + (y1 + y2) / 2;

  const angle = (ux: number, uy: number, vx: number, vy: number) => {
    const n = Math.sqrt(ux * ux + uy * uy);
    const d = Math.sqrt(vx * vx + vy * vy);
    const c = (ux * vx + uy * vy) / (n * d);
    const clamped = Math.max(-1, Math.min(1, c));
    let a = Math.acos(clamped);
    if (ux * vy - uy * vx < 0) a = -a;
    return a;
  };

  const theta1 = angle(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry);
  let dTheta = angle((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry);

  if (!sweep && dTheta > 0) dTheta -= 2 * Math.PI;
  if (sweep && dTheta < 0) dTheta += 2 * Math.PI;

  // Split 最多分成 PI/2 的段
  const segments = Math.ceil(Math.abs(dTheta) / (Math.PI / 2));
  const segAngle = dTheta / segments;

  for (let i = 0; i < segments; i++) {
    const t1 = theta1 + i * segAngle;
    const t2 = t1 + segAngle;
    const alpha =
      (Math.sin(segAngle) * (Math.sqrt(4 + 3 * Math.pow(Math.tan(segAngle / 2), 2)) - 1)) / 3;

    const cos1 = Math.cos(t1),
      sin1 = Math.sin(t1);
    const cos2 = Math.cos(t2),
      sin2 = Math.sin(t2);

    const p1x = cx + rx * cos1;
    const p1y = cy + ry * sin1;
    const p2x = cx + rx * cos2;
    const p2y = cy + ry * sin2;

    const cp1x = p1x - alpha * rx * sin1;
    const cp1y = p1y + alpha * ry * cos1;
    const cp2x = p2x + alpha * rx * sin2;
    const cp2y = p2y - alpha * ry * cos2;

    path.cubicTo(cp1x, cp1y, cp2x, cp2y, p2x, p2y);
  }
}

/**
 * Try 通过标记 SVG
 * 路径字符串来手动构建 CanvasKit 路径。 Handles 可能会拒绝 Handles
 * 的边缘情况（例如，缺少空格、带有“.5”等前导点的数字、相对命令、弧线）。
 */
export function tryManualPathParse(ck: CanvasKit, d: string): Path | null {
  try {
    const path = new ck.Path();
    // Replace NaN/Infinity 为 0，因此命令保留其参数计数。
    const cleaned = d.replace(/-?NaN/g, '0').replace(/-?Infinity/g, '0');
    // Tokenize：分割命令并提取数字
    const tokens = cleaned.match(/[MLCQZAHVSmlcqzahvs]|[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?/g);
    if (!tokens || tokens.length === 0) return null;

    let i = 0;
    let lastCmd = '';
    let cx = 0,
      cy = 0; // 当前点

    while (i < tokens.length) {
      let cmd = tokens[i];
      if (/^[MLCQZAHVSmlcqzahvs]$/.test(cmd)) {
        lastCmd = cmd;
        i++;
      } else if (lastCmd) {
        // Implicit 重复上一个命令（第一对后 m 变为 l）
        cmd = lastCmd === 'M' ? 'L' : lastCmd === 'm' ? 'l' : lastCmd;
      } else {
        i++;
        continue;
      }

      const nums = (count: number): number[] => {
        const result: number[] = [];
        for (let j = 0; j < count && i < tokens.length; j++) {
          const n = parseFloat(tokens[i]);
          if (isNaN(n)) break;
          result.push(n);
          i++;
        }
        return result;
      };

      switch (cmd) {
        case 'M': {
          const p = nums(2);
          if (p.length === 2) {
            path.moveTo(p[0], p[1]);
            cx = p[0];
            cy = p[1];
            lastCmd = 'L';
          }
          break;
        }
        case 'm': {
          const p = nums(2);
          if (p.length === 2) {
            path.moveTo(cx + p[0], cy + p[1]);
            cx += p[0];
            cy += p[1];
            lastCmd = 'l';
          }
          break;
        }
        case 'L': {
          const p = nums(2);
          if (p.length === 2) {
            path.lineTo(p[0], p[1]);
            cx = p[0];
            cy = p[1];
          }
          break;
        }
        case 'l': {
          const p = nums(2);
          if (p.length === 2) {
            path.lineTo(cx + p[0], cy + p[1]);
            cx += p[0];
            cy += p[1];
          }
          break;
        }
        case 'H': {
          const p = nums(1);
          if (p.length === 1) {
            path.lineTo(p[0], cy);
            cx = p[0];
          }
          break;
        }
        case 'h': {
          const p = nums(1);
          if (p.length === 1) {
            path.lineTo(cx + p[0], cy);
            cx += p[0];
          }
          break;
        }
        case 'V': {
          const p = nums(1);
          if (p.length === 1) {
            path.lineTo(cx, p[0]);
            cy = p[0];
          }
          break;
        }
        case 'v': {
          const p = nums(1);
          if (p.length === 1) {
            path.lineTo(cx, cy + p[0]);
            cy += p[0];
          }
          break;
        }
        case 'C': {
          const p = nums(6);
          if (p.length === 6) {
            path.cubicTo(p[0], p[1], p[2], p[3], p[4], p[5]);
            cx = p[4];
            cy = p[5];
          }
          break;
        }
        case 'c': {
          const p = nums(6);
          if (p.length === 6) {
            path.cubicTo(cx + p[0], cy + p[1], cx + p[2], cy + p[3], cx + p[4], cy + p[5]);
            cx += p[4];
            cy += p[5];
          }
          break;
        }
        case 'Q': {
          const p = nums(4);
          if (p.length === 4) {
            path.quadTo(p[0], p[1], p[2], p[3]);
            cx = p[2];
            cy = p[3];
          }
          break;
        }
        case 'q': {
          const p = nums(4);
          if (p.length === 4) {
            path.quadTo(cx + p[0], cy + p[1], cx + p[2], cy + p[3]);
            cx += p[2];
            cy += p[3];
          }
          break;
        }
        case 'S': {
          const p = nums(4);
          if (p.length === 4) {
            path.cubicTo(cx, cy, p[0], p[1], p[2], p[3]);
            cx = p[2];
            cy = p[3];
          }
          break;
        }
        case 's': {
          const p = nums(4);
          if (p.length === 4) {
            path.cubicTo(cx, cy, cx + p[0], cy + p[1], cx + p[2], cy + p[3]);
            cx += p[2];
            cy += p[3];
          }
          break;
        }
        case 'Z':
        case 'z':
          path.close();
          break;
        case 'A':
        case 'a': {
          // Arc：rx、ry、旋转、largeArc、扫描、x、y
          const p = nums(7);
          if (p.length === 7) {
            const [rx, ry, , largeArc, sweep, ex, ey] = p;
            const endX = cmd === 'a' ? cx + ex : ex;
            const endY = cmd === 'a' ? cy + ey : ey;
            if (rx > 0 && ry > 0) {
              arcToCubics(path, cx, cy, rx, ry, largeArc !== 0, sweep !== 0, endX, endY);
            } else {
              path.lineTo(endX, endY);
            }
            cx = endX;
            cy = endY;
          }
          break;
        }
        default:
          i++;
      }
    }

    // Check 如果路径有任何几何图形
    const bounds = path.getBounds();
    if (bounds[2] - bounds[0] < 0.001 && bounds[3] - bounds[1] < 0.001) {
      path.delete();
      return null;
    }
    return path;
  } catch {
    return null;
  }
}
