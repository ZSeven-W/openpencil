import type { PenNode } from '@/types/pen'
import { parseFigFile } from './fig-parser'
import { figmaNodeChangesToPenNodes } from './figma-node-mapper'
import { resolveImageBlobs } from './figma-image-resolver'

/**
 * Quick check: does this HTML string contain Figma clipboard markers?
 * Figma wraps its data in `<!--(figmeta)-->` comment blocks or uses
 * `data-metadata` / `data-buffer` attributes.
 */
export function isFigmaClipboardHtml(html: string): boolean {
  return html.includes('figmeta') || html.includes('data-buffer')
}

// Standard base64 lookup table
const B64_LOOKUP = new Uint8Array(256)
{
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
  for (let i = 0; i < chars.length; i++) B64_LOOKUP[chars.charCodeAt(i)] = i
  // URL-safe variants
  B64_LOOKUP['-'.charCodeAt(0)] = 62
  B64_LOOKUP['_'.charCodeAt(0)] = 63
}

/**
 * Decode a base64 string to Uint8Array without relying on atob.
 * Handles URL-safe alphabet, whitespace, missing padding, and stray characters.
 */
function decodeBase64ToBytes(input: string): Uint8Array {
  // Strip everything except valid base64 characters
  const b64 = input.replace(/[^A-Za-z0-9+/\-_=]/g, '')

  const len = b64.length
  // Compute output byte length (ignoring padding)
  const padding = b64.endsWith('==') ? 2 : b64.endsWith('=') ? 1 : 0
  const byteLen = Math.floor(len * 3 / 4) - padding

  const bytes = new Uint8Array(byteLen)
  let p = 0

  for (let i = 0; i < len; i += 4) {
    const a = B64_LOOKUP[b64.charCodeAt(i)]
    const b = B64_LOOKUP[b64.charCodeAt(i + 1)]
    const c = B64_LOOKUP[b64.charCodeAt(i + 2)]
    const d = B64_LOOKUP[b64.charCodeAt(i + 3)]

    if (p < byteLen) bytes[p++] = (a << 2) | (b >> 4)
    if (p < byteLen) bytes[p++] = ((b & 0x0F) << 4) | (c >> 2)
    if (p < byteLen) bytes[p++] = ((c & 0x03) << 6) | d
  }

  return bytes
}

/**
 * Decode a base64 string to a UTF-8 string.
 */
function decodeBase64(input: string): string {
  const bytes = decodeBase64ToBytes(input)
  return new TextDecoder().decode(bytes)
}

interface FigmaClipboardData {
  meta: Record<string, unknown>
  buffer: ArrayBuffer
}

/**
 * Extract and decode Figma clipboard data from the HTML payload.
 *
 * Figma writes two comment-wrapped, base64-encoded blocks in various formats:
 *   Format A (in HTML comments):
 *     <!--(figmeta)-->BASE64_JSON<!--(figmeta)-->
 *     <!--(figma)-->BASE64_BINARY<!--(figma)-->
 *   Format B (in data attributes):
 *     <span data-metadata="BASE64_JSON"></span>
 *     <span data-buffer="BASE64_BINARY"></span>
 */
export function extractFigmaClipboardData(html: string): FigmaClipboardData | null {
  console.debug('[figma-clipboard] HTML preview (first 500 chars):', html.slice(0, 500))

  let metaB64: string | null = null
  let bufferB64: string | null = null

  // Strategy 1: comment-wrapped format
  // Figma uses <!--(figmeta)BASE64<!--(figmeta)--> (opening lacks -->)
  // or <!--(figmeta)-->BASE64<!--(figmeta)--> (both have -->)
  const metaCommentMatch = html.match(/<!--\(figmeta\)(?:-->)?([\s\S]*?)<!--\(figmeta\)-->/)
  const bufferCommentMatch = html.match(/<!--\(figma\)(?:-->)?([\s\S]*?)<!--\(figma\)-->/)

  if (metaCommentMatch && bufferCommentMatch) {
    console.debug('[figma-clipboard] Matched comment-wrapped format')
    metaB64 = metaCommentMatch[1].trim()
    bufferB64 = bufferCommentMatch[1].trim()
  }

  // Strategy 2: data-attribute format (the comments may be inside attribute values)
  if (!metaB64 || !bufferB64) {
    const attrMetaMatch = html.match(/data-metadata="([^"]*)"/)
    const attrBufferMatch = html.match(/data-buffer="([^"]*)"/)

    if (attrMetaMatch && attrBufferMatch) {
      console.debug('[figma-clipboard] Matched data-attribute format')
      // Strip comment wrappers from attribute values if present.
      // Opening marker may lack --> (e.g. "<!--(figmeta)BASE64<!--(figmeta)-->")
      metaB64 = attrMetaMatch[1]
        .replace(/<!--\(figmeta\)(-->)?/g, '')
        .trim()
      bufferB64 = attrBufferMatch[1]
        .replace(/<!--\(figma\)(-->)?/g, '')
        .trim()
    }
  }

  // Strategy 3: HTML-encoded comment markers inside attributes
  if (!metaB64 || !bufferB64) {
    const encodedMetaMatch = html.match(/&lt;!--\(figmeta\)--&gt;([\s\S]*?)&lt;!--\(figmeta\)--&gt;/)
    const encodedBufferMatch = html.match(/&lt;!--\(figma\)--&gt;([\s\S]*?)&lt;!--\(figma\)--&gt;/)

    if (encodedMetaMatch && encodedBufferMatch) {
      console.debug('[figma-clipboard] Matched HTML-encoded comment format')
      metaB64 = encodedMetaMatch[1].trim()
      bufferB64 = encodedBufferMatch[1].trim()
    }
  }

  if (!metaB64 || !bufferB64) {
    console.warn('[figma-clipboard] No matching extraction strategy.',
      'Has figmeta comment:', /<!--\(figmeta\)-->/.test(html),
      'Has figma comment:', /<!--\(figma\)-->/.test(html),
      'Has data-metadata attr:', /data-metadata=/.test(html),
      'Has data-buffer attr:', /data-buffer=/.test(html),
      'Has encoded figmeta:', /&lt;!--\(figmeta\)/.test(html),
    )
    return null
  }

  console.debug('[figma-clipboard] meta base64 length:', metaB64.length,
    'buffer base64 length:', bufferB64.length)

  try {
    const metaRaw = decodeBase64(metaB64)
    // Trim trailing junk bytes from base64 padding — extract only the JSON object
    const jsonEnd = metaRaw.lastIndexOf('}')
    const metaJson = jsonEnd >= 0 ? metaRaw.slice(0, jsonEnd + 1) : metaRaw
    const meta = JSON.parse(metaJson)
    console.debug('[figma-clipboard] Decoded meta:', meta)

    const bytes = decodeBase64ToBytes(bufferB64)

    console.debug('[figma-clipboard] Decoded buffer:', bytes.byteLength, 'bytes,',
      'first 8 bytes:', Array.from(bytes.slice(0, 8)).map(b => b.toString(16).padStart(2, '0')).join(' '))

    return { meta, buffer: bytes.buffer as ArrayBuffer }
  } catch (err) {
    console.error('[figma-clipboard] Decode error:', err,
      'meta b64 preview:', metaB64.slice(0, 80),
      'buffer b64 preview:', bufferB64.slice(0, 80))
    return null
  }
}

/**
 * Convert a Figma clipboard buffer into PenNodes.
 * The buffer uses the same fig-kiwi binary format as .fig files.
 */
export function figmaClipboardToNodes(
  buffer: ArrayBuffer,
): { nodes: PenNode[]; warnings: string[] } {
  const decoded = parseFigFile(buffer)
  const { nodes, warnings, imageBlobs } = figmaNodeChangesToPenNodes(decoded, 'openpencil')

  // Resolve embedded image blobs to data URLs
  if (imageBlobs.size > 0 || decoded.imageFiles.size > 0) {
    resolveImageBlobs(nodes, imageBlobs, decoded.imageFiles)
  }

  return { nodes, warnings }
}
