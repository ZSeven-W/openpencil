import type { PenNode } from '@/types/pen';
import type { PenFill } from '@/types/styles';
import { enrichNodeLocallyForAIConsumerView } from './consumer-view-enrichment';

export interface CodegenAssetFile {
  id: string;
  relativePath: string;
  zipPath: string;
  mimeType: string;
  bytes: Uint8Array;
  sourceNodeId: string;
  sourceNodeName?: string;
  sourceKind: 'image-node' | 'image-fill';
}

export interface CodegenAssetHint {
  relativePath: string;
  sourceNodeId: string;
  sourceNodeName?: string;
  sourceKind: 'image-node' | 'image-fill';
}

export interface CodegenBundleManifest {
  version: 2;
  framework: string;
  entry: {
    codeFile: string;
  };
  generatedAt: string;
  code: {
    path: string;
    size: number;
    sha256: string;
  };
  assets: Array<{
    id: string;
    relativePath: string;
    zipPath: string;
    mimeType: string;
    size: number;
    sha256: string;
    sourceNodeId: string;
    sourceNodeName?: string;
    sourceKind: 'image-node' | 'image-fill';
  }>;
}

const DATA_URL_PREFIX = 'data:';

function slugifyName(name: string | undefined, fallback: string): string {
  const normalized = (name ?? fallback)
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return normalized || fallback;
}

function inferExtensionFromMimeType(mimeType: string): string {
  const mapped = mimeType.split('/')[1]?.toLowerCase() ?? 'bin';
  if (mapped === 'jpeg') return 'jpg';
  if (mapped === 'svg+xml') return 'svg';
  return mapped.replace(/[^a-z0-9]+/g, '') || 'bin';
}

function decodeBase64(base64: string): Uint8Array {
  if (typeof Buffer !== 'undefined') {
    return Uint8Array.from(Buffer.from(base64, 'base64'));
  }

  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function parseDataUrl(url: string): { mimeType: string; bytes: Uint8Array } | null {
  if (!url.startsWith(DATA_URL_PREFIX)) return null;

  const match = url.match(/^data:([^;,]+);base64,([\s\S]+)$/);
  if (!match) return null;

  const [, mimeType, base64] = match;
  return {
    mimeType,
    bytes: decodeBase64(base64),
  };
}

export async function hashBytesToSha256Hex(bytes: Uint8Array): Promise<string> {
  if (globalThis.crypto?.subtle) {
    const copiedBytes = Uint8Array.from(bytes);
    const digest = await globalThis.crypto.subtle.digest('SHA-256', copiedBytes);
    return Array.from(new Uint8Array(digest))
      .map((byte) => byte.toString(16).padStart(2, '0'))
      .join('');
  }

  const { createHash } = await import('node:crypto');
  return createHash('sha256').update(bytes).digest('hex');
}

/**
 * Extract data URL / base64 payloads from codegen-related nodes into concrete
 * asset file descriptors and rewrite `src` / `fill.url` to stable relative paths.
 *
 * This keeps prompts focused on `./assets/...` paths instead of sending raw base64
 * blobs to the model.
 */
export function extractCodegenAssets(nodes: PenNode[]): {
  nodes: PenNode[];
  assets: CodegenAssetFile[];
} {
  const sanitizedNodes = structuredClone(nodes) as PenNode[];
  const assets: CodegenAssetFile[] = [];
  const seenByDataUrl = new Map<string, CodegenAssetFile>();
  let assetIndex = 1;

  const materializeAsset = (
    sourceUrl: string,
    node: PenNode,
    sourceKind: 'image-node' | 'image-fill',
  ): string => {
    const parsed = parseDataUrl(sourceUrl);
    if (!parsed) return sourceUrl;

    const existing = seenByDataUrl.get(sourceUrl);
    if (existing) return existing.relativePath;

    const ext = inferExtensionFromMimeType(parsed.mimeType);
    const fileStem = slugifyName(node.name, sourceKind === 'image-node' ? 'image' : 'image-fill');
    const fileName = `${fileStem}-${assetIndex}.${ext}`;
    assetIndex++;

    const asset: CodegenAssetFile = {
      id: `asset-${assets.length + 1}`,
      relativePath: `./assets/${fileName}`,
      zipPath: `assets/${fileName}`,
      mimeType: parsed.mimeType,
      bytes: parsed.bytes,
      sourceNodeId: node.id,
      sourceNodeName: node.name,
      sourceKind,
    };
    assets.push(asset);
    seenByDataUrl.set(sourceUrl, asset);
    return asset.relativePath;
  };

  const sanitizeFills = (node: PenNode, fills: PenFill[] | undefined): PenFill[] | undefined => {
    if (!Array.isArray(fills) || fills.length === 0) return fills;

    return fills.map((fill) => {
      if (fill.type !== 'image') return fill;
      return {
        ...fill,
        url: materializeAsset(fill.url, node, 'image-fill'),
      };
    });
  };

  const visit = (node: PenNode): PenNode => {
    const nextNode = { ...node } as PenNode;

    if (nextNode.type === 'image' && typeof nextNode.src === 'string') {
      nextNode.src = materializeAsset(nextNode.src, nextNode, 'image-node');
    }

    if ('fill' in nextNode) {
      nextNode.fill = sanitizeFills(nextNode, nextNode.fill);
    }

    if ('children' in nextNode && Array.isArray(nextNode.children)) {
      nextNode.children = nextNode.children.map(visit);
    }

    return enrichNodeLocallyForAIConsumerView(nextNode);
  };

  return {
    nodes: sanitizedNodes.map(visit),
    assets,
  };
}

export function collectChunkAssetHints(
  chunkNodes: PenNode[],
  assets: CodegenAssetFile[],
): CodegenAssetHint[] {
  const hints: CodegenAssetHint[] = [];
  const seen = new Set<string>();
  const assetByPath = new Map(assets.map((asset) => [asset.relativePath, asset]));

  const visit = (node: PenNode) => {
    const pushHint = (relativePath: string, sourceKind: 'image-node' | 'image-fill') => {
      if (seen.has(relativePath)) return;
      const asset = assetByPath.get(relativePath);
      if (!asset) return;

      seen.add(relativePath);
      hints.push({
        relativePath,
        sourceNodeId: node.id,
        sourceNodeName: node.name,
        sourceKind,
      });
    };

    if (node.type === 'image' && typeof node.src === 'string' && node.src.startsWith('./assets/')) {
      pushHint(node.src, 'image-node');
    }

    if ('fill' in node && Array.isArray(node.fill)) {
      for (const fill of node.fill) {
        if (fill.type !== 'image' || typeof fill.url !== 'string') continue;
        if (fill.url.startsWith('./assets/')) {
          pushHint(fill.url, 'image-fill');
        }
      }
    }

    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) visit(child);
    }
  };

  for (const node of chunkNodes) visit(node);
  return hints;
}

export async function buildCodegenBundleManifest(options: {
  framework: string;
  codeFile: string;
  codeBytes: Uint8Array;
  assets: CodegenAssetFile[];
}): Promise<CodegenBundleManifest> {
  const codeSha256 = await hashBytesToSha256Hex(options.codeBytes);
  const assetsWithHashes = await Promise.all(
    options.assets.map(async (asset) => ({
      id: asset.id,
      relativePath: asset.relativePath,
      zipPath: asset.zipPath,
      mimeType: asset.mimeType,
      size: asset.bytes.byteLength,
      sha256: await hashBytesToSha256Hex(asset.bytes),
      sourceNodeId: asset.sourceNodeId,
      sourceNodeName: asset.sourceNodeName,
      sourceKind: asset.sourceKind,
    })),
  );

  return {
    version: 2,
    framework: options.framework,
    entry: {
      codeFile: options.codeFile,
    },
    generatedAt: new Date().toISOString(),
    code: {
      path: options.codeFile,
      size: options.codeBytes.byteLength,
      sha256: codeSha256,
    },
    assets: assetsWithHashes,
  };
}
