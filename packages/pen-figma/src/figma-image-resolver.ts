import type { PenNode } from '@zseven-w/pen-types';
import type { ImageFill } from '@zseven-w/pen-types';

/**
 * Resolve
 * __blob:n 和 __hash:<hex> 在 PenNode 树中引用数据 URLs，使用从 .fig 文件中提取的图像 blob 和 ZIP
 图像文件。
 */
export function resolveImageBlobs(
  nodes: PenNode[],
  imageBlobs: Map<number, Uint8Array>,
  imageFiles?: Map<string, Uint8Array>,
): number {
  if (imageBlobs.size === 0 && (!imageFiles || imageFiles.size === 0)) return 0;

  // Convert blob 到数据 URLs
  const dataUrls = new Map<number, string>();
  for (const [index, bytes] of imageBlobs) {
    dataUrls.set(index, blobToDataUrl(bytes));
  }

  // Convert 基于哈希的图像文件到数据 URLs
  const hashDataUrls = new Map<string, string>();
  if (imageFiles) {
    for (const [hash, bytes] of imageFiles) {
      hashDataUrls.set(hash, blobToDataUrl(bytes));
    }
  }

  let resolved = 0;
  for (const node of nodes) {
    resolved += patchNode(node, dataUrls, hashDataUrls);
  }
  return resolved;
}

function blobToDataUrl(bytes: Uint8Array): string {
  // Detect MIME 魔法字节类型
  let mime = 'image/png';
  if (bytes[0] === 0xff && bytes[1] === 0xd8) {
    mime = 'image/jpeg';
  } else if (bytes[0] === 0x47 && bytes[1] === 0x49) {
    mime = 'image/gif';
  } else if (bytes[0] === 0x52 && bytes[1] === 0x49) {
    mime = 'image/webp';
  }

  // Convert 转 base64
  let binary = '';
  const len = bytes.byteLength;
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  const base64 = btoa(binary);
  return `data:${mime};base64,${base64}`;
}

function resolveRef(
  src: string,
  dataUrls: Map<number, string>,
  hashDataUrls: Map<string, string>,
): string | null {
  if (src.startsWith('__blob:')) {
    const index = parseInt(src.slice(7), 10);
    return dataUrls.get(index) ?? null;
  }
  if (src.startsWith('__hash:')) {
    const hash = src.slice(7);
    return hashDataUrls.get(hash) ?? null;
  }
  return null;
}

function patchNode(
  node: PenNode,
  dataUrls: Map<number, string>,
  hashDataUrls: Map<string, string>,
): number {
  let resolved = 0;

  // Patch ImageNode src
  if (
    node.type === 'image' &&
    node.src &&
    (node.src.startsWith('__blob:') || node.src.startsWith('__hash:'))
  ) {
    const url = resolveRef(node.src, dataUrls, hashDataUrls);
    if (url) {
      node.src = url;
      resolved++;
    }
  }

  // Patch image fills
  if ('fill' in node && Array.isArray(node.fill)) {
    for (const fill of node.fill) {
      if (fill.type === 'image') {
        const imgFill = fill as ImageFill;
        if (
          imgFill.url &&
          (imgFill.url.startsWith('__blob:') || imgFill.url.startsWith('__hash:'))
        ) {
          const url = resolveRef(imgFill.url, dataUrls, hashDataUrls);
          if (url) {
            imgFill.url = url;
            resolved++;
          }
        }
      }
    }
  }

  // Recurse into children
  if ('children' in node && Array.isArray(node.children)) {
    for (const child of node.children) {
      resolved += patchNode(child, dataUrls, hashDataUrls);
    }
  }

  return resolved;
}
