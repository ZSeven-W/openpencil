import type { CanvasKit, Path } from 'canvaskit-wasm'

/**
 * Normalize SVG path data for CanvasKit's parser:
 * - Add spaces between command letters and numbers
 * - Handle negative-sign number separators (e.g. "10-5" → "10 -5")
 * - Normalize comma separators to spaces
 * - Separate concatenated arc flags (e.g. "a2 2 0 012 2" → "a2 2 0 0 1 2 2")
 */
export function sanitizeSvgPath(d: string): string {
  let result = d
    // Add space between command letter and following number/sign
    .replace(/([MLCQZAHVSmlcqzahvsTt])([0-9.+-])/g, '$1 $2')
    // Add space between digit and following negative sign (number separator)
    .replace(/(\d)-/g, '$1 -')
    // Replace commas with spaces
    .replace(/,/g, ' ')
    // Collapse multiple spaces
    .replace(/\s+/g, ' ')
    .trim()

  // Separate concatenated arc flags: in SVG arc commands, the large-arc and
  // sweep flags are single digits (0 or 1) that may be concatenated with each
  // other and with the following number. e.g. "a2 2 0 012 2" → "a2 2 0 0 1 2 2"
  result = result.replace(
    /([aA])\s*([\d.e+-]+)\s+([\d.e+-]+)\s+([\d.e+-]+)\s+([01])([01])([\d.+-])/g,
    '$1 $2 $3 $4 $5 $6 $7',
  )
  // Handle the case where all three (rotation + flags) are concatenated without spaces,
  // e.g. "a4 4 0100-8" where 0100 = rotation=0, large-arc=1, sweep=0, then 0 is start of x
  result = result.replace(
    /([aA])\s*([\d.e+-]+)\s+([\d.e+-]+)\s+(\d)([01])([01])([\d.+-])/g,
    '$1 $2 $3 $4 $5 $6 $7',
  )

  return result
}

/** Returns true if the path string contains NaN or Infinity values. */
export function hasInvalidNumbers(d: string): boolean {
  return /NaN|Infinity/i.test(d)
}

/**
 * Convert an SVG arc segment to cubic bezier curves and add them to the path.
 * Based on the W3C SVG implementation note for arc-to-cubic conversion.
 */
function arcToCubics(
  path: Path,
  x1: number, y1: number,
  rxIn: number, ryIn: number,
  largeArc: boolean, sweep: boolean,
  x2: number, y2: number,
): void {
  // Degenerate: start == end
  if (x1 === x2 && y1 === y2) return

  let rx = Math.abs(rxIn)
  let ry = Math.abs(ryIn)

  const dx = (x1 - x2) / 2
  const dy = (y1 - y2) / 2
  // Simplified: ignore rotation (most icons use rotation=0)
  const x1p = dx
  const y1p = dy

  // Correct radii
  let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry)
  if (lambda > 1) {
    const s = Math.sqrt(lambda)
    rx *= s
    ry *= s
  }

  const rxSq = rx * rx
  const rySq = ry * ry
  const x1pSq = x1p * x1p
  const y1pSq = y1p * y1p

  let sq = (rxSq * rySq - rxSq * y1pSq - rySq * x1pSq) / (rxSq * y1pSq + rySq * x1pSq)
  if (sq < 0) sq = 0
  let root = Math.sqrt(sq)
  if (largeArc === sweep) root = -root

  const cxp = root * rx * y1p / ry
  const cyp = -root * ry * x1p / rx

  const cx = cxp + (x1 + x2) / 2
  const cy = cyp + (y1 + y2) / 2

  const angle = (ux: number, uy: number, vx: number, vy: number) => {
    const n = Math.sqrt(ux * ux + uy * uy)
    const d = Math.sqrt(vx * vx + vy * vy)
    const c = (ux * vx + uy * vy) / (n * d)
    const clamped = Math.max(-1, Math.min(1, c))
    let a = Math.acos(clamped)
    if (ux * vy - uy * vx < 0) a = -a
    return a
  }

  const theta1 = angle(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry)
  let dTheta = angle(
    (x1p - cxp) / rx, (y1p - cyp) / ry,
    (-x1p - cxp) / rx, (-y1p - cyp) / ry,
  )

  if (!sweep && dTheta > 0) dTheta -= 2 * Math.PI
  if (sweep && dTheta < 0) dTheta += 2 * Math.PI

  // Split into segments of at most PI/2
  const segments = Math.ceil(Math.abs(dTheta) / (Math.PI / 2))
  const segAngle = dTheta / segments

  for (let i = 0; i < segments; i++) {
    const t1 = theta1 + i * segAngle
    const t2 = t1 + segAngle
    const alpha = Math.sin(segAngle) * (Math.sqrt(4 + 3 * Math.pow(Math.tan(segAngle / 2), 2)) - 1) / 3

    const cos1 = Math.cos(t1), sin1 = Math.sin(t1)
    const cos2 = Math.cos(t2), sin2 = Math.sin(t2)

    const p1x = cx + rx * cos1
    const p1y = cy + ry * sin1
    const p2x = cx + rx * cos2
    const p2y = cy + ry * sin2

    const cp1x = p1x - alpha * rx * sin1
    const cp1y = p1y + alpha * ry * cos1
    const cp2x = p2x + alpha * rx * sin2
    const cp2y = p2y - alpha * ry * cos2

    path.cubicTo(cp1x, cp1y, cp2x, cp2y, p2x, p2y)
  }
}

/**
 * Try building a CanvasKit path manually by tokenizing the SVG path string.
 * Handles edge cases that MakeFromSVGString may reject (e.g. missing spaces,
 * numbers with leading dots like ".5", relative commands, arcs).
 */
export function tryManualPathParse(ck: CanvasKit, d: string): Path | null {
  try {
    const path = new ck.Path()
    // Replace NaN/Infinity with 0 so commands keep their parameter count.
    // This produces degenerate but valid paths (e.g. a horizontal line at y=0)
    // which is better than dropping the entire command.
    const cleaned = d.replace(/-?NaN/g, '0').replace(/-?Infinity/g, '0')
    // Tokenize: split on commands and extract numbers
    const tokens = cleaned.match(/[MLCQZAHVSmlcqzahvs]|[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?/g)
    if (!tokens || tokens.length === 0) return null

    let i = 0
    let lastCmd = ''
    let cx = 0, cy = 0 // current point

    while (i < tokens.length) {
      let cmd = tokens[i]
      if (/^[MLCQZAHVSmlcqzahvs]$/.test(cmd)) {
        lastCmd = cmd
        i++
      } else if (lastCmd) {
        // Implicit repeat of last command (M becomes L after first pair)
        cmd = lastCmd === 'M' ? 'L' : lastCmd === 'm' ? 'l' : lastCmd
      } else {
        i++
        continue
      }

      const nums = (count: number): number[] => {
        const result: number[] = []
        for (let j = 0; j < count && i < tokens.length; j++) {
          const n = parseFloat(tokens[i])
          if (isNaN(n)) break
          result.push(n)
          i++
        }
        return result
      }

      switch (cmd) {
        case 'M': { const p = nums(2); if (p.length === 2) { path.moveTo(p[0], p[1]); cx = p[0]; cy = p[1]; lastCmd = 'L' } break }
        case 'm': { const p = nums(2); if (p.length === 2) { path.moveTo(cx + p[0], cy + p[1]); cx += p[0]; cy += p[1]; lastCmd = 'l' } break }
        case 'L': { const p = nums(2); if (p.length === 2) { path.lineTo(p[0], p[1]); cx = p[0]; cy = p[1] } break }
        case 'l': { const p = nums(2); if (p.length === 2) { path.lineTo(cx + p[0], cy + p[1]); cx += p[0]; cy += p[1] } break }
        case 'H': { const p = nums(1); if (p.length === 1) { path.lineTo(p[0], cy); cx = p[0] } break }
        case 'h': { const p = nums(1); if (p.length === 1) { path.lineTo(cx + p[0], cy); cx += p[0] } break }
        case 'V': { const p = nums(1); if (p.length === 1) { path.lineTo(cx, p[0]); cy = p[0] } break }
        case 'v': { const p = nums(1); if (p.length === 1) { path.lineTo(cx, cy + p[0]); cy += p[0] } break }
        case 'C': { const p = nums(6); if (p.length === 6) { path.cubicTo(p[0], p[1], p[2], p[3], p[4], p[5]); cx = p[4]; cy = p[5] } break }
        case 'c': { const p = nums(6); if (p.length === 6) { path.cubicTo(cx+p[0], cy+p[1], cx+p[2], cy+p[3], cx+p[4], cy+p[5]); cx += p[4]; cy += p[5] } break }
        case 'Q': { const p = nums(4); if (p.length === 4) { path.quadTo(p[0], p[1], p[2], p[3]); cx = p[2]; cy = p[3] } break }
        case 'q': { const p = nums(4); if (p.length === 4) { path.quadTo(cx+p[0], cy+p[1], cx+p[2], cy+p[3]); cx += p[2]; cy += p[3] } break }
        case 'S': { const p = nums(4); if (p.length === 4) { path.cubicTo(cx, cy, p[0], p[1], p[2], p[3]); cx = p[2]; cy = p[3] } break }
        case 's': { const p = nums(4); if (p.length === 4) { path.cubicTo(cx, cy, cx+p[0], cy+p[1], cx+p[2], cy+p[3]); cx += p[2]; cy += p[3] } break }
        case 'Z': case 'z': path.close(); break
        case 'A': case 'a': {
          // Arc: rx, ry, rotation, largeArc, sweep, x, y
          // Convert SVG arc to cubic bezier approximation via CanvasKit's conic API
          const p = nums(7)
          if (p.length === 7) {
            const [rx, ry, , largeArc, sweep, ex, ey] = p
            const endX = cmd === 'a' ? cx + ex : ex
            const endY = cmd === 'a' ? cy + ey : ey
            if (rx > 0 && ry > 0) {
              arcToCubics(path, cx, cy, rx, ry, largeArc !== 0, sweep !== 0, endX, endY)
            } else {
              path.lineTo(endX, endY)
            }
            cx = endX
            cy = endY
          }
          break
        }
        default: i++
      }
    }

    // Check if path has any geometry
    const bounds = path.getBounds()
    if (bounds[2] - bounds[0] < 0.001 && bounds[3] - bounds[1] < 0.001) {
      path.delete()
      return null
    }
    return path
  } catch {
    return null
  }
}
