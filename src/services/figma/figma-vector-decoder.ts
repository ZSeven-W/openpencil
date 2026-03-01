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
        parts.push(`M${r(x)} ${r(y)}`)
        break
      }
      case 0x02: { // lineTo
        if (offset + 8 > blob.length) return joinParts(parts)
        const x = view.getFloat32(offset, true); offset += 4
        const y = view.getFloat32(offset, true); offset += 4
        parts.push(`L${r(x)} ${r(y)}`)
        break
      }
      case 0x03: { // quadTo
        if (offset + 16 > blob.length) return joinParts(parts)
        const cpx = view.getFloat32(offset, true); offset += 4
        const cpy = view.getFloat32(offset, true); offset += 4
        const x   = view.getFloat32(offset, true); offset += 4
        const y   = view.getFloat32(offset, true); offset += 4
        parts.push(`Q${r(cpx)} ${r(cpy)} ${r(x)} ${r(y)}`)
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
        parts.push(`C${r(cp1x)} ${r(cp1y)} ${r(cp2x)} ${r(cp2y)} ${r(x)} ${r(y)}`)
        break
      }
      default:
        // Unknown command — stop decoding
        return joinParts(parts)
    }
  }

  return joinParts(parts)
}

/** Round to 2 decimal places for compact SVG path data. */
function r(n: number): string {
  return Math.abs(n) < 0.005 ? '0' : parseFloat(n.toFixed(2)).toString()
}

function joinParts(parts: string[]): string | null {
  return parts.length > 1 ? parts.join(' ') : null
}

/**
 * Try to decode vector path data from a Figma node's fill/stroke geometry blobs.
 * Scales coordinates from normalizedSize to actual node size if needed.
 */
export function decodeFigmaVectorPath(
  figma: FigmaNodeChange,
  blobs: (Uint8Array | string)[],
): string | null {
  // Try fillGeometry first, then strokeGeometry
  const geometries = figma.fillGeometry ?? figma.strokeGeometry
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

  const rawPath = pathParts.join(' ')

  // Scale from normalizedSize to actual node size if they differ
  const normSize = figma.vectorData?.normalizedSize
  const actualSize = figma.size
  if (normSize && actualSize) {
    const sx = actualSize.x / normSize.x
    const sy = actualSize.y / normSize.y
    if (Math.abs(sx - 1) > 0.01 || Math.abs(sy - 1) > 0.01) {
      return scaleSvgPath(rawPath, sx, sy)
    }
  }

  return rawPath
}

/** Scale all coordinates in an SVG path string. */
function scaleSvgPath(d: string, sx: number, sy: number): string {
  // Tokenize: commands and numbers
  const tokens = d.match(/[MLCQZmlcqz]|-?\d+\.?\d*/g)
  if (!tokens) return d

  const result: string[] = []
  let i = 0

  while (i < tokens.length) {
    const token = tokens[i]
    if (/^[MLCQZmlcqz]$/.test(token)) {
      result.push(token)
      i++
      const cmd = token.toUpperCase()
      const count = cmd === 'M' || cmd === 'L' ? 2 : cmd === 'Q' ? 4 : cmd === 'C' ? 6 : 0
      for (let j = 0; j < count && i < tokens.length; j++) {
        const val = parseFloat(tokens[i])
        result.push(r(j % 2 === 0 ? val * sx : val * sy))
        i++
      }
    } else {
      result.push(token)
      i++
    }
  }

  return result.join(' ')
}
