import { lstat, readFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { createHash } from 'node:crypto';
import { liveDocument } from './document-format.mjs';

const MAX_IMAGE_BYTES = 8 * 1024 * 1024;
const MAX_DOCUMENT_BYTES = 32 * 1024 * 1024;

function mimeType(bytes) {
  if (bytes.length >= 33 && bytes.subarray(0, 8).toString('hex') === '89504e470d0a1a0a' &&
      bytes.toString('ascii', 12, 16) === 'IHDR') return 'image/png';
  if (bytes.length >= 4 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) return 'image/jpeg';
  if (bytes.length >= 16 && bytes.toString('ascii', 0, 4) === 'RIFF' &&
      bytes.toString('ascii', 8, 12) === 'WEBP') return 'image/webp';
  throw new Error('Use a PNG, JPEG or WebP image file. SVG and non-image files are not imported.');
}

function target(live, pageId, nodeId) {
  const document = liveDocument(live.document);
  const page = document.pages?.length ? document.pages[live.activePageIndex] : document;
  if (!page || (document.pages?.length ? page.id : '0') !== pageId) {
    throw new Error('The active page changed. Read the canvas and confirm the image target again.');
  }
  const stack = [...(page.children || [])];
  while (stack.length) {
    const node = stack.pop();
    if (node.id === nodeId) {
      if (node.type !== 'image') throw new Error('The target must be an existing image node; other node types are preserved.');
      return { document, node };
    }
    stack.push(...(node.children || []));
  }
  throw new Error('The image target no longer exists on this page. No image was imported.');
}

// Import bytes, not file URLs: WASM and reopened documents cannot rely on a
// generated asset remaining in the host's cache or temporary directory.
export async function importCanvasImage(session, { filePath, nodeId, pageId, expectedVersion } = {}) {
  if (typeof filePath !== 'string' || !isAbsolute(filePath)) throw new Error('Use an absolute local image file path.');
  if (typeof nodeId !== 'string' || !nodeId.trim() || typeof pageId !== 'string' || !pageId.trim() ||
      !Number.isSafeInteger(expectedVersion) || expectedVersion < 0) {
    throw new Error('Provide the exact image nodeId, pageId and expectedVersion from the current canvas.');
  }
  const info = await lstat(filePath);
  if (!info.isFile() || info.isSymbolicLink() || !info.size || info.size > MAX_IMAGE_BYTES) {
    throw new Error('Use a regular image file of at most 8 MiB.');
  }
  const bytes = await readFile(filePath);
  if (!bytes.length || bytes.length > MAX_IMAGE_BYTES) throw new Error('Use an image file of at most 8 MiB.');
  const mime = mimeType(bytes);
  const src = `data:${mime};base64,${bytes.toString('base64')}`;
  const live = await session.document();
  if (live.version !== expectedVersion) throw new Error('The canvas changed. Read it and confirm the image target before importing.');
  const current = target(live, pageId, nodeId);
  current.node.src = src;
  current.node.imagePrompt = null;
  current.node.imageSearchQuery = null;
  if (Buffer.byteLength(JSON.stringify(current.document)) > MAX_DOCUMENT_BYTES) {
    throw new Error('Embedding this image would exceed the 32 MiB document limit. Use a smaller asset.');
  }
  const reply = await session.forward({ jsonrpc: '2.0', id: 'canvas-import-image', method: 'tools/call',
    params: { name: 'update_node', arguments: { nodeId, pageId,
      data: { src, imagePrompt: null, imageSearchQuery: null } } } });
  if (reply?.error || !reply?.result || reply.result.isError) {
    throw new Error('Image import was not confirmed. Inspect the canvas before repeating it; the write was not retried.');
  }
  const after = await session.document();
  if (target(after, pageId, nodeId).node.src !== src) {
    throw new Error('Image import readback did not match. Inspect the target before making another change.');
  }
  return { nodeId, pageId, version: after.version, bytes: bytes.length, mimeType: mime,
    sha256: createHash('sha256').update(bytes).digest('hex'), embedded: true, saved: false };
}
