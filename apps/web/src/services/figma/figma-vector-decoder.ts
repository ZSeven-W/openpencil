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

  if (!geometries || geometries.length === 0) {
    // Try to decode from vectorData.vectorNetworkBlob as fallback
    return decodeVectorNetworkBlob(figma, blobs)
  }

  const pathParts: string[] = []

  for (const geom of geometries) {
    if (geom.commandsBlob == null) continue
    const blob = blobs[geom.commandsBlob]
    if (!blob || typeof blob === 'string') continue
    const decoded = decodeFigmaPathBlob(blob)
    if (decoded) pathParts.push(decoded)
  }

  if (pathParts.length === 0) {
    // Try vectorNetworkBlob fallback
    const vnPath = decodeVectorNetworkBlob(figma, blobs)
    if (vnPath) return vnPath
    return null
  }

  // fillGeometry/strokeGeometry coordinates are already in the node's local
  // coordinate space (0..size.x, 0..size.y). Do NOT scale by normalizedSize —
  // that applies only to vectorNetworkBlob, which is not used here.
  return pathParts.join(' ')
}

/**
 * Decode Figma's vectorNetworkBlob (VectorNetwork) as a fallback when
 * fill/stroke geometry blobs are not available.
 *
 * The vectorNetwork blob format:
 *   [4 bytes LE] vertex count
 *   For each vertex:  [4 bytes float32 LE x] [4 bytes float32 LE y]
 *   [4 bytes LE] segment count
 *   For each segment:
 *     [4 bytes LE] start vertex index
 *     [4 bytes LE] end vertex index
 *     [4 bytes float32 LE] tangentStart.x
 *     [4 bytes float32 LE] tangentStart.y
 *     [4 bytes float32 LE] tangentEnd.x
 *     [4 bytes float32 LE] tangentEnd.y
 */
function decodeVectorNetworkBlob(
  figma: FigmaNodeChange,
  blobs: (Uint8Array | string)[],
): string | null {
  const blobIdx = figma.vectorData?.vectorNetworkBlob
  if (blobIdx == null) return null
  const blob = blobs[blobIdx]
  if (!blob || typeof blob === 'string' || blob.length < 8) return null

  const buf = new ArrayBuffer(blob.byteLength)
  new Uint8Array(buf).set(blob)
  const view = new DataView(buf)
  let offset = 0

  try {
    // Read vertices
    const vertexCount = view.getUint32(offset, true); offset += 4
    if (vertexCount > 100000 || offset + vertexCount * 8 > blob.length) return null

    const vertices: { x: number; y: number }[] = []
    for (let i = 0; i < vertexCount; i++) {
      const x = view.getFloat32(offset, true); offset += 4
      const y = view.getFloat32(offset, true); offset += 4
      vertices.push({ x, y })
    }

    if (offset + 4 > blob.length) return null
    // Read segments
    const segmentCount = view.getUint32(offset, true); offset += 4
    if (segmentCount > 100000) return null

    // Build adjacency list: for each vertex, which segments start from it
    const segments: {
      start: number; end: number
      ts: { x: number; y: number }; te: { x: number; y: number }
    }[] = []

    for (let i = 0; i < segmentCount; i++) {
      if (offset + 24 > blob.length) break
      const startIdx = view.getUint32(offset, true); offset += 4
      const endIdx = view.getUint32(offset, true); offset += 4
      const tsx = view.getFloat32(offset, true); offset += 4
      const tsy = view.getFloat32(offset, true); offset += 4
      const tex = view.getFloat32(offset, true); offset += 4
      const tey = view.getFloat32(offset, true); offset += 4

      if (startIdx < vertexCount && endIdx < vertexCount) {
        segments.push({
          start: startIdx, end: endIdx,
          ts: { x: tsx, y: tsy }, te: { x: tex, y: tey },
        })
      }
    }

    if (segments.length === 0 || vertices.length === 0) return null

    // Scale from normalizedSize to actual node size
    const normW = figma.vectorData?.normalizedSize?.x ?? 1
    const normH = figma.vectorData?.normalizedSize?.y ?? 1
    const nodeW = figma.size?.x ?? normW
    const nodeH = figma.size?.y ?? normH
    const sx = normW > 0.001 ? nodeW / normW : 1
    const sy = normH > 0.001 ? nodeH / normH : 1

    // Convert segments to SVG path commands
    // Simple approach: each segment becomes an independent moveTo + curveTo/lineTo
    const parts: string[] = []
    const used = new Set<number>()

    // Build adjacency for chain walking
    const adj = new Map<number, number[]>()
    for (let i = 0; i < segments.length; i++) {
      const s = segments[i]
      if (!adj.has(s.start)) adj.set(s.start, [])
      adj.get(s.start)!.push(i)
    }

    // Walk chains starting from each unused segment
    for (let i = 0; i < segments.length; i++) {
      if (used.has(i)) continue

      const seg = segments[i]
      const sv = vertices[seg.start]
      parts.push(`M${r(sv.x * sx)} ${r(sv.y * sy)}`)
      used.add(i)

      // Emit this segment
      emitSegment(seg, vertices, sx, sy, parts)

      // Follow chain
      let current = seg.end
      let found = true
      while (found) {
        found = false
        const nexts = adj.get(current)
        if (nexts) {
          for (const ni of nexts) {
            if (used.has(ni)) continue
            used.add(ni)
            emitSegment(segments[ni], vertices, sx, sy, parts)
            current = segments[ni].end
            found = true
            break
          }
        }
      }

      // Check if path is closed
      if (current === seg.start) parts.push('Z')
    }

    const result = parts.join(' ')
    return result || null
  } catch {
    return null
  }
}

function emitSegment(
  seg: { start: number; end: number; ts: { x: number; y: number }; te: { x: number; y: number } },
  vertices: { x: number; y: number }[],
  sx: number, sy: number,
  parts: string[],
): void {
  const sv = vertices[seg.start]
  const ev = vertices[seg.end]
  const isStraight =
    Math.abs(seg.ts.x) < 0.0001 && Math.abs(seg.ts.y) < 0.0001 &&
    Math.abs(seg.te.x) < 0.0001 && Math.abs(seg.te.y) < 0.0001

  if (isStraight) {
    parts.push(`L${r(ev.x * sx)} ${r(ev.y * sy)}`)
  } else {
    // Tangents are relative offsets from start/end vertices
    const cp1x = (sv.x + seg.ts.x) * sx
    const cp1y = (sv.y + seg.ts.y) * sy
    const cp2x = (ev.x + seg.te.x) * sx
    const cp2y = (ev.y + seg.te.y) * sy
    parts.push(`C${r(cp1x)} ${r(cp1y)} ${r(cp2x)} ${r(cp2y)} ${r(ev.x * sx)} ${r(ev.y * sy)}`)
  }
}