import type { PenNode } from '@/types/pen'
import type { ImageFill } from '@/types/styles'

/**
 * Resolve __blob:N references in the PenNode tree to data URLs
 * using extracted image blobs from the .fig file.
 */
export function resolveImageBlobs(
  nodes: PenNode[],
  imageBlobs: Map<number, Uint8Array>,
): number {
  if (imageBlobs.size === 0) return 0

  // Convert blobs to data URLs
  const dataUrls = new Map<number, string>()
  for (const [index, bytes] of imageBlobs) {
    dataUrls.set(index, blobToDataUrl(bytes))
  }

  let resolved = 0
  for (const node of nodes) {
    resolved += patchNode(node, dataUrls)
  }
  return resolved
}

function blobToDataUrl(bytes: Uint8Array): string {
  // Detect MIME type from magic bytes
  let mime = 'image/png'
  if (bytes[0] === 0xFF && bytes[1] === 0xD8) {
    mime = 'image/jpeg'
  } else if (bytes[0] === 0x47 && bytes[1] === 0x49) {
    mime = 'image/gif'
  } else if (bytes[0] === 0x52 && bytes[1] === 0x49) {
    mime = 'image/webp'
  }

  // Convert to base64
  let binary = ''
  const len = bytes.byteLength
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  const base64 = btoa(binary)
  return `data:${mime};base64,${base64}`
}

function patchNode(node: PenNode, dataUrls: Map<number, string>): number {
  let resolved = 0

  // Patch ImageNode src
  if (node.type === 'image' && node.src?.startsWith('__blob:')) {
    const index = parseInt(node.src.slice(7), 10)
    const url = dataUrls.get(index)
    if (url) {
      node.src = url
      resolved++
    }
  }

  // Patch image fills
  if ('fill' in node && Array.isArray(node.fill)) {
    for (const fill of node.fill) {
      if (fill.type === 'image') {
        const imgFill = fill as ImageFill
        if (imgFill.url?.startsWith('__blob:')) {
          const index = parseInt(imgFill.url.slice(7), 10)
          const url = dataUrls.get(index)
          if (url) {
            imgFill.url = url
            resolved++
          }
        }
      }
    }
  }

  // Recurse into children
  if ('children' in node && Array.isArray(node.children)) {
    for (const child of node.children) {
      resolved += patchNode(child, dataUrls)
    }
  }

  return resolved
}
