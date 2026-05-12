import type { Framework } from '@zseven-w/pen-types';
import type {
  CloudCodeGeneration,
  CloudCodeGenerationDetail,
  CloudCodegenFile,
  CodegenTarget,
  SaveCodeGenerationInput,
  SaveCodegenAssetInput,
} from '@/types/cloud';
import type { CodegenAssetFile } from '@/services/ai/codegen-assets';
import { buildCodegenFiles } from '@/services/ai/codegen-files';
import { cloudFetch, cloudFetchRaw } from './cloud-fetch';

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

export async function getCodeGenerationHistoryDetail(
  generationId: string,
): Promise<CloudCodeGenerationDetail> {
  const res = await cloudFetch<{ data: CloudCodeGenerationDetail }>(
    `/api/cloud/code-generations/${generationId}`,
  );
  return res.data;
}

export async function listCodeGenerationFiles(generationId: string): Promise<CloudCodegenFile[]> {
  const res = await cloudFetch<{ data: CloudCodegenFile[] }>(
    `/api/cloud/code-generations/${generationId}/files`,
  );
  return res.data;
}

export async function saveCodeGenerationHistory(
  input: SaveCodeGenerationHistoryInput,
): Promise<CloudCodeGenerationDetail> {
  const files =
    input.files ?? buildCodegenFiles({ framework: input.framework, code: input.finalCode ?? '' });
  const body: SaveCodeGenerationInput = {
    ...input,
    entryFile: input.entryFile ?? files[0]?.path,
    assets: serializeCodegenAssets(input.assets),
    files,
  };
  const res = await cloudFetch<{ data: CloudCodeGenerationDetail }>('/api/cloud/code-generations', {
    method: 'POST',
    body: JSON.stringify(body),
  });
  return res.data;
}

export async function promoteCodeGenerationHistory(
  generationId: string,
): Promise<CloudCodeGeneration> {
  const res = await cloudFetch<{ data: CloudCodeGeneration }>(
    `/api/cloud/code-generations/${generationId}/promote`,
    { method: 'POST' },
  );
  return res.data;
}

export async function deleteCodeGenerationHistory(generationId: string): Promise<void> {
  await cloudFetchRaw(`/api/cloud/code-generations/${generationId}`, {
    method: 'DELETE',
  });
}

function getFileNameFromContentDisposition(value: string | null): string | null {
  if (!value) return null;
  const utf8Match = /filename\*=UTF-8''([^;]+)/i.exec(value);
  if (utf8Match?.[1]) return decodeURIComponent(utf8Match[1]);
  const quotedMatch = /filename="([^"]+)"/i.exec(value);
  if (quotedMatch?.[1]) return quotedMatch[1];
  const plainMatch = /filename=([^;]+)/i.exec(value);
  return plainMatch?.[1]?.trim() ?? null;
}

export async function exportCodeGenerationZip(generationId: string): Promise<{
  blob: Blob;
  fileName: string;
}> {
  const response = await cloudFetchRaw(`/api/cloud/code-generations/${generationId}/export-zip`, {
    method: 'POST',
  });
  return {
    blob: await response.blob(),
    fileName:
      getFileNameFromContentDisposition(response.headers.get('Content-Disposition')) ??
      'design-codegen.zip',
  };
}
