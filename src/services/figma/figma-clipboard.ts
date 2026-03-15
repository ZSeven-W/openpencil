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
 *
 * @param buffer  The decoded binary buffer from the Figma clipboard.
 * @param html    Optional full clipboard HTML — when provided, styled content
 *                outside the binary comments is parsed to supplement missing
 *                style properties (colors, fonts) on the binary-parsed nodes.
 */
export function figmaClipboardToNodes(
  buffer: ArrayBuffer,
  html?: string,
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

  // Enrich nodes with style hints extracted from the clipboard HTML.
  // Figma clipboard HTML contains styled elements (with inline CSS) that
  // may carry color/font information lost during binary parsing (e.g. when
  // shared style nodes are not included in the clipboard data).
  if (html) {
    const hints = parseClipboardHtmlStyles(html)
    if (hints.size > 0) {
      enrichNodesFromHtmlHints(nodes, hints)
    }
  }

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
    // Convert standalone image nodes with unresolved references to rectangles
    if (node.type === 'image' && node.src && (node.src.startsWith('__blob:') || node.src.startsWith('__hash:'))) {
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
    // Fix unresolved image fills on rectangles/ellipses/frames —
    // __blob: and __hash: are internal references that the image loader
    // cannot fetch; replace with a placeholder solid fill.
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

// ---------------------------------------------------------------------------
// HTML style extraction — parse the styled portion of Figma clipboard HTML
// to recover color/font information that may be missing from the binary data.
// ---------------------------------------------------------------------------

interface HtmlStyleHint {
  color?: string
  fontFamily?: string
  fontSize?: number
  fontWeight?: number
  backgroundColor?: string
}

/**
 * Convert a CSS color value (hex, rgb, rgba) to a #RRGGBB(AA) hex string.
 */
function cssColorToHex(css: string): string | undefined {
  const c = css.trim()
  if (c.startsWith('#')) {
    // Normalize 3-digit to 6-digit hex
    if (c.length === 4) {
      return `#${c[1]}${c[1]}${c[2]}${c[2]}${c[3]}${c[3]}`
    }
    return c
  }
  const rgbaMatch = c.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+)\s*)?\)/)
  if (rgbaMatch) {
    const r = parseInt(rgbaMatch[1]).toString(16).padStart(2, '0')
    const g = parseInt(rgbaMatch[2]).toString(16).padStart(2, '0')
    const b = parseInt(rgbaMatch[3]).toString(16).padStart(2, '0')
    if (rgbaMatch[4] !== undefined) {
      const a = Math.round(parseFloat(rgbaMatch[4]) * 255)
      if (a < 255) return `#${r}${g}${b}${a.toString(16).padStart(2, '0')}`
    }
    return `#${r}${g}${b}`
  }
  return undefined
}

/**
 * Decode HTML entities (&#NN; and &amp;/&lt;/etc.) in text content.
 */
function decodeHtmlEntities(text: string): string {
  return text
    .replace(/&#(\d+);/g, (_, code) => String.fromCharCode(parseInt(code)))
    .replace(/&#x([0-9a-fA-F]+);/g, (_, hex) => String.fromCharCode(parseInt(hex, 16)))
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&nbsp;/g, ' ')
}

/**
 * Parse the styled HTML portion of a Figma clipboard to extract text style hints.
 * Figma clipboard HTML contains styled elements (p, span, div) with inline CSS
 * in addition to the binary data in comment blocks.  These inline styles carry
 * resolved color/font values that are sometimes missing from the binary format
 * (e.g. when a node references a shared style that is not included in the
 * clipboard data).
 *
 * Returns a map keyed by normalized text content → style properties.
 */
function parseClipboardHtmlStyles(html: string): Map<string, HtmlStyleHint> {
  // Remove the binary data comment blocks to isolate the styled HTML content
  const cleanHtml = html
    .replace(/<!--\(figmeta\)[\s\S]*?<!--\(figmeta\)-->/g, '')
    .replace(/<!--\(figma\)[\s\S]*?<!--\(figma\)-->/g, '')

  const hints = new Map<string, HtmlStyleHint>()

  // Match elements with style attributes and text content.
  // Captures: style attribute value, text content between tags.
  const elemRegex = /style="([^"]*)"[^>]*>([^<]+)</gi
  let match
  while ((match = elemRegex.exec(cleanHtml)) !== null) {
    const styleAttr = match[1]
    const rawText = decodeHtmlEntities(match[2]).trim()
    if (!rawText || rawText.length > 200) continue

    const hint: HtmlStyleHint = {}

    // color (text color) — avoid matching background-color
    const colorMatch = styleAttr.match(/(?:^|;\s*)color:\s*((?:rgba?\([^)]+\)|#[0-9a-fA-F]{3,8}))/)
    if (colorMatch) hint.color = cssColorToHex(colorMatch[1])

    // font-family
    const fontMatch = styleAttr.match(/font-family:\s*([^;]+)/)
    if (fontMatch) {
      const family = fontMatch[1].trim().replace(/['"]/g, '').split(',')[0].trim()
      if (family) hint.fontFamily = family
    }

    // font-size
    const sizeMatch = styleAttr.match(/font-size:\s*(\d+(?:\.\d+)?)px/)
    if (sizeMatch) hint.fontSize = parseFloat(sizeMatch[1])

    // font-weight
    const weightMatch = styleAttr.match(/font-weight:\s*(\d+)/)
    if (weightMatch) hint.fontWeight = parseInt(weightMatch[1])

    // background-color (for div/frame enrichment)
    const bgMatch = styleAttr.match(/background-color:\s*((?:rgba?\([^)]+\)|#[0-9a-fA-F]{3,8}))/)
    if (bgMatch) hint.backgroundColor = cssColorToHex(bgMatch[1])

    if (Object.keys(hint).length > 0) {
      // Use first occurrence — later duplicates may be nested/overridden
      if (!hints.has(rawText)) {
        hints.set(rawText, hint)
      }
    }
  }

  return hints
}

/**
 * Walk the PenNode tree and fill in missing style properties using hints
 * extracted from the clipboard HTML.  Only fills in properties that are
 * undefined/missing — explicit values from the binary parser are never
 * overwritten.
 */
function enrichNodesFromHtmlHints(
  nodes: PenNode[],
  hints: Map<string, HtmlStyleHint>,
): void {
  for (const node of nodes) {
    if (node.type === 'text') {
      // Build plain text content for lookup
      const content = typeof node.content === 'string'
        ? node.content
        : Array.isArray(node.content)
          ? node.content.map(s => s.text).join('')
          : ''
      const trimmed = content.trim()
      if (!trimmed) continue

      // Try exact match first, then try individual lines
      const hint = hints.get(trimmed) ?? findPartialHint(trimmed, hints)
      if (hint) {
        // Fill in missing text color
        if (!node.fill && hint.color) {
          node.fill = [{ type: 'solid', color: hint.color }]
        }
        // Fill in missing font properties
        if (!node.fontFamily && hint.fontFamily) {
          node.fontFamily = hint.fontFamily
        }
        if (!node.fontSize && hint.fontSize) {
          node.fontSize = hint.fontSize
        }
        if (!node.fontWeight && hint.fontWeight) {
          node.fontWeight = hint.fontWeight
        }
      }
    }

    // For frames/rectangles without fill, check if HTML has background-color
    if ((node.type === 'frame' || node.type === 'rectangle') && !node.fill) {
      const name = node.name?.trim()
      if (name) {
        const hint = hints.get(name)
        if (hint?.backgroundColor) {
          node.fill = [{ type: 'solid', color: hint.backgroundColor }]
        }
      }
    }

    // Recurse into children
    if ('children' in node && Array.isArray(node.children)) {
      enrichNodesFromHtmlHints(node.children, hints)
    }
  }
}

/**
 * Try to find a matching hint for text that may span multiple lines or
 * may be a subset of a longer HTML text.
 */
function findPartialHint(
  text: string,
  hints: Map<string, HtmlStyleHint>,
): HtmlStyleHint | undefined {
  // Check if text starts with any hint key
  for (const [key, hint] of hints) {
    if (text.startsWith(key) || key.startsWith(text)) {
      return hint
    }
  }
  return undefined
}
