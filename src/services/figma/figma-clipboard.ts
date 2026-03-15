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
  let metaB64: string | null = null
  let bufferB64: string | null = null

  // Strategy 1: comment-wrapped format
  // Figma uses <!--(figmeta)BASE64<!--(figmeta)--> (opening lacks -->)
  // or <!--(figmeta)-->BASE64<!--(figmeta)--> (both have -->)
  const metaCommentMatch = html.match(/<!--\(figmeta\)(?:-->)?([\s\S]*?)<!--\(figmeta\)-->/)
  const bufferCommentMatch = html.match(/<!--\(figma\)(?:-->)?([\s\S]*?)<!--\(figma\)-->/)

  if (metaCommentMatch && bufferCommentMatch) {
    metaB64 = metaCommentMatch[1].trim()
    bufferB64 = bufferCommentMatch[1].trim()
  }

  // Strategy 2: data-attribute format (the comments may be inside attribute values)
  if (!metaB64 || !bufferB64) {
    const attrMetaMatch = html.match(/data-metadata="([^"]*)"/)
    const attrBufferMatch = html.match(/data-buffer="([^"]*)"/)

    if (attrMetaMatch && attrBufferMatch) {
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
      metaB64 = encodedMetaMatch[1].trim()
      bufferB64 = encodedBufferMatch[1].trim()
    }
  }

  if (!metaB64 || !bufferB64) return null

  try {
    const metaRaw = decodeBase64(metaB64)
    // Trim trailing junk bytes from base64 padding — extract only the JSON object
    const jsonEnd = metaRaw.lastIndexOf('}')
    const metaJson = jsonEnd >= 0 ? metaRaw.slice(0, jsonEnd + 1) : metaRaw
    const meta = JSON.parse(metaJson)
    const bytes = decodeBase64ToBytes(bufferB64)
    return { meta, buffer: bytes.buffer as ArrayBuffer }
  } catch {
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
  // Use 'preserve' layout mode (same as .fig file import) so that:
  // 1. Auto-layout children are reversed to correct flow order
  // 2. Image nodes get numeric pixel dimensions instead of sizing strings
  const { nodes, warnings, imageBlobs } = figmaNodeChangesToPenNodes(decoded, 'preserve')

  // Resolve embedded image blobs to data URLs
  if (imageBlobs.size > 0 || decoded.imageFiles.size > 0) {
    resolveImageBlobs(nodes, imageBlobs, decoded.imageFiles)
  }

  // Handle unresolved image references — clipboard data often lacks image
  // binary data.  Convert unresolvable image nodes to placeholder rectangles.
  fixUnresolvedImages(nodes)

  return { nodes, warnings }
}

/**
 * Walk the node tree and convert image nodes with unresolved __blob:/__hash:
 * references into placeholder rectangles.  Clipboard data often lacks the
 * actual image binary, so leaving these as image nodes with broken src would
 * render as invisible/broken elements.
 */
function fixUnresolvedImages(nodes: PenNode[]): void {
  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i]
    if (node.type === 'image' && node.src && (node.src.startsWith('__blob:') || node.src.startsWith('__hash:'))) {
      // Convert to a placeholder rectangle preserving position and size
      const rect: PenNode = {
        type: 'rectangle',
        id: node.id,
        name: node.name,
        x: node.x,
        y: node.y,
        width: node.width,
        height: node.height,
        cornerRadius: node.cornerRadius,
        opacity: node.opacity,
        fill: [{ type: 'solid', color: '#E5E7EB' }],
      }
      nodes[i] = rect
    }
    // Also fix image fills on other node types
    if ('fill' in node && Array.isArray(node.fill)) {
      for (let j = node.fill.length - 1; j >= 0; j--) {
        const fill = node.fill[j]
        if (fill.type === 'image' && 'url' in fill) {
          const url = (fill as any).url as string
          if (url?.startsWith('__blob:') || url?.startsWith('__hash:')) {
            node.fill[j] = { type: 'solid', color: '#E5E7EB' }
          }
        }
      }
    }
    // Recurse into children
    if ('children' in node && Array.isArray(node.children)) {
      fixUnresolvedImages(node.children)
    }
  }
}
