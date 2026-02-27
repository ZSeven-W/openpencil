import { ByteBuffer, compileSchema, decodeBinarySchema } from 'kiwi-schema'
import * as UZIP from 'uzip'
import { decompress as zstdDecompress } from 'fzstd'
import type { FigmaDecodedFile } from './figma-types'

// Magic bytes for "fig-kiwi"
const FIG_KIWI_MAGIC = [102, 105, 103, 45, 107, 105, 119, 105]
// Zstandard magic bytes: 0x28 0xB5 0x2F 0xFD
const ZSTD_MAGIC = [0x28, 0xB5, 0x2F, 0xFD]
// PNG magic bytes
const PNG_MAGIC_0 = 137
const PNG_MAGIC_1 = 80

const int32 = new Int32Array(1)
const uint8 = new Uint8Array(int32.buffer)
const uint32 = new Uint32Array(int32.buffer)

function transfer8to32(fileByte: Uint8Array, start: number): void {
  uint8[0] = fileByte[start]
  uint8[1] = fileByte[start + 1]
  uint8[2] = fileByte[start + 2]
  uint8[3] = fileByte[start + 3]
}

function readUint32(fileByte: Uint8Array, start: number): number {
  transfer8to32(fileByte, start)
  return uint32[0]
}

function hasFigKiwiMagic(bytes: Uint8Array): boolean {
  for (let i = 0; i < FIG_KIWI_MAGIC.length; i++) {
    if (bytes[i] !== FIG_KIWI_MAGIC[i]) return false
  }
  return true
}

function isZstd(bytes: Uint8Array): boolean {
  return (
    bytes.length >= 4 &&
    bytes[0] === ZSTD_MAGIC[0] &&
    bytes[1] === ZSTD_MAGIC[1] &&
    bytes[2] === ZSTD_MAGIC[2] &&
    bytes[3] === ZSTD_MAGIC[3]
  )
}

function isPng(bytes: Uint8Array): boolean {
  return bytes.length >= 2 && bytes[0] === PNG_MAGIC_0 && bytes[1] === PNG_MAGIC_1
}

/**
 * Decompress a chunk using the appropriate algorithm.
 * Figma uses deflate for the schema chunk and may use zstd for the data chunk.
 */
function decompressChunk(bytes: Uint8Array): Uint8Array {
  // Don't decompress PNG image data
  if (isPng(bytes)) return bytes

  // Try zstd first if magic matches
  if (isZstd(bytes)) {
    return zstdDecompress(bytes)
  }

  // Try deflate (inflateRaw)
  try {
    return UZIP.inflateRaw(bytes) as Uint8Array<ArrayBuffer>
  } catch {
    // If deflate fails, try zstd as fallback (some files may not have magic)
    try {
      return zstdDecompress(bytes)
    } catch {
      // Return raw bytes if neither works
      return bytes
    }
  }
}

/**
 * Split a .fig file buffer into schema and data binary parts.
 */
function figToBinaryParts(fileBuffer: ArrayBuffer): Uint8Array[] {
  let fileByte = new Uint8Array(fileBuffer)

  // If not starting with "fig-kiwi", it's a ZIP archive containing canvas.fig
  if (!hasFigKiwiMagic(fileByte)) {
    let unzipped: Record<string, Uint8Array>
    try {
      unzipped = UZIP.parse(fileBuffer)
    } catch (e) {
      throw new Error(
        `Invalid .fig file: could not unzip (${e instanceof Error ? e.message : 'unknown error'})`
      )
    }
    const canvasFile = unzipped['canvas.fig']
    if (!canvasFile) {
      const keys = Object.keys(unzipped)
      throw new Error(
        `Invalid .fig file: no canvas.fig found in archive. Contents: [${keys.join(', ')}]`
      )
    }
    fileBuffer = canvasFile.buffer as ArrayBuffer
    fileByte = new Uint8Array(fileBuffer)
  }

  if (!hasFigKiwiMagic(fileByte)) {
    throw new Error('Invalid .fig file: missing fig-kiwi header after extraction')
  }

  // Skip 8 bytes magic + 4 bytes delimiter
  let start = 8
  readUint32(fileByte, start)
  start += 4

  const result: Uint8Array[] = []
  while (start < fileByte.length) {
    const chunkSize = readUint32(fileByte, start)
    start += 4

    if (chunkSize === 0 || start + chunkSize > fileByte.length) break

    const rawChunk = fileByte.slice(start, start + chunkSize)
    const decompressed = decompressChunk(rawChunk)
    result.push(decompressed)
    start += chunkSize
  }

  return result
}

/* eslint-disable @typescript-eslint/no-explicit-any */

/**
 * Find the decode function for the root message type in the compiled schema.
 * Figma's .fig files use a root type called "Message", so we look for
 * `decodeMessage` first, then fall back to any `decode*` method.
 */
function findDecoder(
  schemaHelper: Record<string, any>
): (bb: any) => any {
  // Primary: Figma uses "Message" as root type
  if (typeof schemaHelper.decodeMessage === 'function') {
    return schemaHelper.decodeMessage.bind(schemaHelper)
  }

  // Fallback: find any decode* method
  for (const key of Object.keys(schemaHelper)) {
    if (key.startsWith('decode') && typeof schemaHelper[key] === 'function') {
      return schemaHelper[key].bind(schemaHelper)
    }
  }

  throw new Error(
    `No decode method found in schema. Available keys: [${Object.keys(schemaHelper).join(', ')}]`
  )
}

/**
 * Parse a .fig file ArrayBuffer into decoded JSON.
 */
export function parseFigFile(fileBuffer: ArrayBuffer): FigmaDecodedFile {
  const parts = figToBinaryParts(fileBuffer)

  if (parts.length < 2) {
    throw new Error(
      `Invalid .fig file: expected at least 2 binary parts, got ${parts.length}`
    )
  }

  const [schemaByte, dataByte] = parts

  let schema: any
  try {
    const schemaBB = new ByteBuffer(schemaByte)
    schema = decodeBinarySchema(schemaBB)
  } catch (e) {
    throw new Error(
      `Failed to decode .fig schema: ${e instanceof Error ? e.message : 'unknown error'}`
    )
  }

  let schemaHelper: Record<string, any>
  try {
    schemaHelper = compileSchema(schema) as Record<string, any>
  } catch (e) {
    throw new Error(
      `Failed to compile .fig schema: ${e instanceof Error ? e.message : 'unknown error'}`
    )
  }

  const decoder = findDecoder(schemaHelper)

  let raw: any
  try {
    const dataBB = new ByteBuffer(dataByte)
    raw = decoder(dataBB)
  } catch (e) {
    throw new Error(
      `Failed to decode .fig data: ${e instanceof Error ? e.message : 'unknown error'}`
    )
  }

  if (!raw || typeof raw !== 'object') {
    throw new Error('Decoded .fig data is empty or invalid')
  }

  // Extract nodeChanges
  const nodeChanges = raw.nodeChanges ?? []
  if (nodeChanges.length === 0) {
    // Some schemas may use a different field name — search for arrays with guid objects
    for (const key of Object.keys(raw)) {
      if (Array.isArray(raw[key]) && raw[key].length > 0 && raw[key][0]?.guid) {
        return {
          nodeChanges: raw[key],
          blobs: extractBlobs(raw),
        }
      }
    }
  }

  return {
    nodeChanges,
    blobs: extractBlobs(raw),
  }
}

function extractBlobs(raw: any): (Uint8Array | string)[] {
  const blobs: (Uint8Array | string)[] = []
  if (!raw.blobs) return blobs

  for (const blob of raw.blobs) {
    if (blob?.bytes instanceof Uint8Array) {
      blobs.push(blob.bytes)
    } else if (typeof blob === 'string') {
      blobs.push(blob)
    } else {
      blobs.push(new Uint8Array(0))
    }
  }
  return blobs
}
