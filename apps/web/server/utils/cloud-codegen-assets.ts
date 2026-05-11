import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';
import type { CodegenAssetManifestEntry, SaveCodegenAssetInput } from '../../src/types/cloud';
import { CLOUD_DOCUMENT_STORAGE_BUCKET } from './cloud-document-storage';

const SIGNED_URL_EXPIRES_IN_SECONDS = 60 * 60;

function base64ToBytes(value: string): Uint8Array {
  if (typeof Buffer !== 'undefined') {
    return Uint8Array.from(Buffer.from(value, 'base64'));
  }

  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function normalizeAssetPath(path: string): string {
  return path
    .replace(/^\.?\//, '')
    .split('/')
    .map((part) => part.replace(/[^a-zA-Z0-9._-]+/g, '-'))
    .filter(Boolean)
    .join('/');
}

export async function persistCodeGenerationAssets(input: {
  supabase: SupabaseClient;
  userId: string;
  fileId: string;
  generationId: string;
  assets: SaveCodegenAssetInput[];
}): Promise<CodegenAssetManifestEntry[]> {
  const manifest: CodegenAssetManifestEntry[] = [];

  for (const asset of input.assets) {
    const bytes = base64ToBytes(asset.bytesBase64);
    const zipPath = normalizeAssetPath(asset.zipPath || asset.relativePath || asset.id);
    const storagePath = `${input.userId}/${input.fileId}/code-generations/${input.generationId}/${zipPath}`;

    const uploaded = await input.supabase.storage
      .from(CLOUD_DOCUMENT_STORAGE_BUCKET)
      .upload(storagePath, bytes, {
        contentType: asset.mimeType,
        upsert: false,
      });
    if (uploaded.error) {
      throw createError({ statusCode: 500, statusMessage: uploaded.error.message });
    }

    const recorded = await input.supabase.from('file_assets').insert({
      file_id: input.fileId,
      owner_id: input.userId,
      kind: 'codegen_asset',
      bucket: CLOUD_DOCUMENT_STORAGE_BUCKET,
      path: storagePath,
      mime_type: asset.mimeType,
      size_bytes: bytes.byteLength,
      metadata: {
        generationId: input.generationId,
        assetId: asset.id,
        relativePath: asset.relativePath,
        zipPath: asset.zipPath,
        sourceNodeId: asset.sourceNodeId,
        sourceNodeName: asset.sourceNodeName,
        sourceKind: asset.sourceKind,
      },
    });
    if (recorded.error) {
      throw createError({ statusCode: 500, statusMessage: recorded.error.message });
    }

    manifest.push({
      id: asset.id,
      relativePath: asset.relativePath,
      zipPath: asset.zipPath,
      mimeType: asset.mimeType,
      sizeBytes: bytes.byteLength,
      storagePath,
      sourceNodeId: asset.sourceNodeId,
      sourceNodeName: asset.sourceNodeName,
      sourceKind: asset.sourceKind,
    });
  }

  return manifest;
}

export async function addSignedUrlsToCodegenManifest(
  supabase: SupabaseClient,
  manifest: CodegenAssetManifestEntry[] | null | undefined,
): Promise<CodegenAssetManifestEntry[]> {
  const entries = manifest ?? [];
  return Promise.all(
    entries.map(async (entry) => {
      if (!entry.storagePath) return entry;

      const signed = await supabase.storage
        .from(CLOUD_DOCUMENT_STORAGE_BUCKET)
        .createSignedUrl(entry.storagePath, SIGNED_URL_EXPIRES_IN_SECONDS);
      if (signed.error || !signed.data?.signedUrl) return entry;

      return {
        ...entry,
        signedUrl: signed.data.signedUrl,
      };
    }),
  );
}
