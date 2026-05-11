import type { Framework } from '@zseven-w/pen-types';
import type {
  CloudCodeGeneration,
  CloudCodeGenerationDetail,
  CodegenTarget,
  SaveCodeGenerationInput,
  SaveCodegenAssetInput,
} from '@/types/cloud';
import type { CodegenAssetFile } from '@/services/ai/codegen-assets';
import { cloudFetch } from './cloud-fetch';

type SaveCodeGenerationHistoryInput = Omit<SaveCodeGenerationInput, 'assets'> & {
  assets: CodegenAssetFile[];
};

function stableHash(value: string): string {
  let hash = 2166136261;
  for (let i = 0; i < value.length; i += 1) {
    hash ^= value.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, '0');
}

export function buildCodegenTarget(input: {
  pageId: string | null;
  selectedIds: string[];
}): CodegenTarget {
  const pageId = input.pageId ?? 'default';
  const nodeIds = [...input.selectedIds].sort();
  if (nodeIds.length === 0) {
    return {
      pageId,
      targetKind: 'page',
      nodeIds: [],
      targetHash: stableHash(`${pageId}:page`),
    };
  }
  return {
    pageId,
    targetKind: 'selection',
    nodeIds,
    targetHash: stableHash(`${pageId}:selection:${nodeIds.join(',')}`),
  };
}

function bytesToBase64(bytes: Uint8Array): string {
  if (typeof Buffer !== 'undefined') {
    return Buffer.from(bytes).toString('base64');
  }

  let binary = '';
  const chunkSize = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    const chunk = bytes.subarray(offset, offset + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return btoa(binary);
}

export function serializeCodegenAssets(assets: CodegenAssetFile[]): SaveCodegenAssetInput[] {
  return assets.map((asset) => ({
    id: asset.id,
    relativePath: asset.relativePath,
    zipPath: asset.zipPath,
    mimeType: asset.mimeType,
    bytesBase64: bytesToBase64(asset.bytes),
    sourceNodeId: asset.sourceNodeId,
    sourceNodeName: asset.sourceNodeName,
    sourceKind: asset.sourceKind,
  }));
}

export async function listCodeGenerationHistory(input: {
  fileId: string;
  framework: Framework;
  target: CodegenTarget;
  limit?: number;
}): Promise<CloudCodeGeneration[]> {
  const params = new URLSearchParams({
    fileId: input.fileId,
    framework: input.framework,
    pageId: input.target.pageId,
    targetKind: input.target.targetKind,
    targetHash: input.target.targetHash,
    limit: String(input.limit ?? 20),
  });
  const res = await cloudFetch<{ data: CloudCodeGeneration[] }>(
    `/api/cloud/code-generations?${params.toString()}`,
  );
  return res.data;
}

export async function saveCodeGenerationHistory(
  input: SaveCodeGenerationHistoryInput,
): Promise<CloudCodeGenerationDetail> {
  const body: SaveCodeGenerationInput = {
    ...input,
    assets: serializeCodegenAssets(input.assets),
  };
  const res = await cloudFetch<{ data: CloudCodeGenerationDetail }>('/api/cloud/code-generations', {
    method: 'POST',
    body: JSON.stringify(body),
  });
  return res.data;
}
