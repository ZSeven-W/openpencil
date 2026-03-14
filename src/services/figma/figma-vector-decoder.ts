import type { FigmaNodeChange } from './figma-types'

/**
 * Decode Figma binary path blob to SVG path `d` string.
 * Binary format: sequence of commands, each starting with a command byte:
 *   0x00 = closePath (Z) — 0 floats
 *   0x01 = moveTo (M)    — 2 float32 LE (x, y)
 *   0x02 = lineTo (L)    — 2 float32 LE (x, y)
 *   0x04 = cubicTo (C)   — 6 float32 LE (cp1x, cp1y, cp2x, cp2y, x, y)
 *   0x03 = quadTo (Q)    — 4 float32 LE (cpx, cpy, x, y)
 */
function decodeFigmaPathBlob(blob: Uint8Array): string | null {
  if (blob.length < 9) return null // minimum: 1 cmd byte + 2 float32

  const buf = new ArrayBuffer(blob.byteLength)
  new Uint8Array(buf).set(blob)
  const view = new DataView(buf)

  const parts: string[] = []
  let offset = 0

  while (offset < blob.length) {
    const cmd = blob[offset]
    offset += 1

    switch (cmd) {
      case 0x00: // close
        parts.push('Z')
        break
      case 0x01: { // moveTo
        if (offset + 8 > blob.length) return joinParts(parts)
        const x = view.getFloat32(offset, true); offset += 4
        const y = view.getFloat32(offset, true); offset += 4
        if (!hasNaN(x, y)) parts.push(`M${r(x)} ${r(y)}`)
        break
      }
      case 0x02: { // lineTo
        if (offset + 8 > blob.length) return joinParts(parts)
        const x = view.getFloat32(offset, true); offset += 4
        const y = view.getFloat32(offset, true); offset += 4
        if (!hasNaN(x, y)) parts.push(`L${r(x)} ${r(y)}`)
        break
      }
      case 0x03: { // quadTo
        if (offset + 16 > blob.length) return joinParts(parts)
        const cpx = view.getFloat32(offset, true); offset += 4
        const cpy = view.getFloat32(offset, true); offset += 4
        const x   = view.getFloat32(offset, true); offset += 4
        const y   = view.getFloat32(offset, true); offset += 4
        if (!hasNaN(cpx, cpy, x, y)) parts.push(`Q${r(cpx)} ${r(cpy)} ${r(x)} ${r(y)}`)
        break
      }
      case 0x04: { // cubicTo
        if (offset + 24 > blob.length) return joinParts(parts)
        const cp1x = view.getFloat32(offset, true); offset += 4
        const cp1y = view.getFloat32(offset, true); offset += 4
        const cp2x = view.getFloat32(offset, true); offset += 4
        const cp2y = view.getFloat32(offset, true); offset += 4
        const x    = view.getFloat32(offset, true); offset += 4
        const y    = view.getFloat32(offset, true); offset += 4
        if (!hasNaN(cp1x, cp1y, cp2x, cp2y, x, y)) parts.push(`C${r(cp1x)} ${r(cp1y)} ${r(cp2x)} ${r(cp2y)} ${r(x)} ${r(y)}`)
        break
      }
      default:
        // Unknown command — stop decoding
        return joinParts(parts)
    }
  }

  return joinParts(parts)
}

/** Round to 4 decimal places for accurate SVG path data. */
function r(n: number): string {
  return Math.abs(n) < 0.00005 ? '0' : parseFloat(n.toFixed(4)).toString()
}

/** Check if any float is NaN/Infinity. */
function hasNaN(...vals: number[]): boolean {
  for (const v of vals) { if (!Number.isFinite(v)) return true }
  return false
}

function joinParts(parts: string[]): string | null {
  return parts.length > 0 ? parts.join(' ') : null
}

export interface PathBounds {
  minX: number; minY: number; maxX: number; maxY: number
}

/**
 * Compute approximate bounding box of an SVG path string from its coordinates.
 * Uses control points (not curve extrema), which is sufficient for layout purposes.
 */
export function computeSvgPathBounds(d: string): PathBounds | null {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  const cmds = d.match(/[MLCQZ][^MLCQZ]*/gi)
  if (!cmds) return null
  for (const cmd of cmds) {
    const letter = cmd[0].toUpperCase()
    if (letter === 'Z') continue
    const coords = cmd.slice(1).trim().match(/-?\d+\.?\d*/g)
    if (!coords) continue
    const vals = coords.map(Number)
    for (let i = 0; i < vals.length - 1; i += 2) {
      const x = vals[i], y = vals[i + 1]
      if (Number.isFinite(x) && Number.isFinite(y)) {
        minX = Math.min(minX, x)
        minY = Math.min(minY, y)
        maxX = Math.max(maxX, x)
        maxY = Math.max(maxY, y)
      }
    }
  }
  if (!Number.isFinite(minX)) return null
  return { minX, minY, maxX, maxY }
}

/**
 * Try to decode vector path data from a Figma node's fill/stroke geometry blobs.
 * Scales coordinates from normalizedSize to actual node size if needed.
 */
export function decodeFigmaVectorPath(
  figma: FigmaNodeChange,
  blobs: (Uint8Array | string)[],
): string | null {
  // For stroke-only vectors (e.g. Lucide icons), prefer strokeGeometry which
  // contains the original centerline path.  fillGeometry for stroke-only vectors
  // is the expanded stroke outline — stroking it again produces double thickness.
  const hasVisibleFills = figma.fillPaints?.some((p) => p.visible !== false)
  const hasVisibleStrokes = figma.strokePaints?.some((p) => p.visible !== false)
  const geometries = (!hasVisibleFills && hasVisibleStrokes)
    ? (figma.strokeGeometry ?? figma.fillGeometry)
    : (figma.fillGeometry ?? figma.strokeGeometry)
  if (!geometries || geometries.length === 0) return null

  const pathParts: string[] = []

  for (const geom of geometries) {
    if (geom.commandsBlob == null) continue
    const blob = blobs[geom.commandsBlob]
    if (!blob || typeof blob === 'string') continue
    const decoded = decodeFigmaPathBlob(blob)
    if (decoded) pathParts.push(decoded)
  }

  if (pathParts.length === 0) return null

  // fillGeometry/strokeGeometry coordinates are already in the node's local
  // coordinate space (0..size.x, 0..size.y). Do NOT scale by normalizedSize —
  // that applies only to vectorNetworkBlob, which is not used here.
  return pathParts.join(' ')
}