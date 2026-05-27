import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';
import { encode as encodeZip } from 'uzip';
import type { CloudCodeGenerationDetail, CodegenAssetManifestEntry } from '../../src/types/cloud';
import type { CodegenAssetFile } from '../../src/services/ai/codegen-assets';
import { buildCodegenBundleManifest } from '../../src/services/ai/codegen-assets';
import { buildCodegenFiles, getDefaultCodegenFilePath } from '../../src/services/ai/codegen-files';
import { CLOUD_DOCUMENT_STORAGE_BUCKET } from './cloud-document-storage';
import { getCodeGenerationDetail } from './cloud-code-generation-records';

const textEncoder = new TextEncoder();

function encodeText(content: string): Uint8Array {
  return textEncoder.encode(content);
}

function serializeJson(value: unknown): Uint8Array {
  return encodeText(JSON.stringify(value, null, 2));
}

function addZipEntry(entries: Record<string, Uint8Array>, path: string, bytes: Uint8Array) {
  if (entries[path]) return;
  entries[path] = bytes;
}

function buildReadme(detail: CloudCodeGenerationDetail, files: Array<{ path: string }>): string {
  const qualityStatus =
    typeof detail.metadata?.qualityStatus === 'string' ? detail.metadata.qualityStatus : 'unknown';
  const commands =
    detail.framework === 'uniapp'
      ? ['Install dependencies in your UniApp project.', 'Run with HBuilderX or your UniApp CLI.']
      : detail.framework === 'vue'
        ? ['Copy the generated files into a Vue 3 project.', 'Run your project build command.']
        : detail.framework === 'html'
          ? ['Open design.html in a browser.']
          : ['Copy the generated files into the target project.'];

  return [
    '# OpenPencil Generated Code',
    '',
    `Framework: ${detail.framework}`,
    `Quality status: ${qualityStatus}`,
    `Document revision: ${detail.documentRevision}`,
    '',
    '## Files',
    '',
    ...files.map((file) => `- ${file.path}`),
    '',
    '## Run / integrate',
    '',
    ...commands.map((command) => `- ${command}`),
    '',
    '## Notes',
    '',
    '- Replace mock/static data with real business data as needed.',
    '- Keep asset paths under assets/ when moving files.',
  ].join('\n');
}

async function loadExportableAssets(input: {
  supabase: SupabaseClient;
  assetsManifest: CodegenAssetManifestEntry[];
}): Promise<CodegenAssetFile[]> {
  const assets: CodegenAssetFile[] = [];

  for (const asset of input.assetsManifest) {
    if (!asset.storagePath) {
      throw createError({
        statusCode: 500,
        statusMessage: `Missing stored asset path for code generation asset ${asset.id}`,
      });
    }

    const downloaded = await input.supabase.storage
      .from(CLOUD_DOCUMENT_STORAGE_BUCKET)
      .download(asset.storagePath);
    if (downloaded.error || !downloaded.data) {
      throw createError({
        statusCode: 500,
        statusMessage:
          downloaded.error?.message ?? `Failed to download code generation asset ${asset.id}`,
      });
    }

    assets.push({
      id: asset.id,
      relativePath: asset.relativePath,
      zipPath: asset.zipPath,
      mimeType: asset.mimeType,
      bytes: new Uint8Array(await downloaded.data.arrayBuffer()),
      sourceNodeId: asset.sourceNodeId ?? '',
      sourceNodeName: asset.sourceNodeName,
      sourceKind: asset.sourceKind as 'image-node' | 'image-fill',
    });
  }

  return assets;
}

export interface BuildCodeGenerationExportZipInput {
  supabase: SupabaseClient;
  userId: string;
  generationId: string;
}

export interface CodeGenerationExportZipResult {
  fileName: string;
  bytes: ArrayBuffer;
  detail: CloudCodeGenerationDetail;
}

export async function buildCodeGenerationExportZip(
  input: BuildCodeGenerationExportZipInput,
): Promise<CodeGenerationExportZipResult> {
  const detail = await getCodeGenerationDetail({
    supabase: input.supabase,
    userId: input.userId,
    generationId: input.generationId,
  });

  const storedFiles = detail.files ?? [];
  const files =
    storedFiles.length > 0
      ? storedFiles
      : buildCodegenFiles({
          framework: detail.framework,
          code: detail.finalCode ?? '',
        });
  const codeFilePath = getDefaultCodegenFilePath(detail.framework);
  const codeBytes = encodeText(detail.finalCode ?? '');
  const assets = await loadExportableAssets({
    supabase: input.supabase,
    assetsManifest: detail.assetsManifest,
  });
  const manifest = await buildCodegenBundleManifest({
    framework: detail.framework,
    codeFile: codeFilePath,
    codeBytes,
    assets,
  });
  const manifestFileName =
    detail.framework === 'uniapp' ? 'openpencil-codegen-manifest.json' : 'manifest.json';

  const zipEntries: Record<string, Uint8Array> = {};
  addZipEntry(zipEntries, codeFilePath, codeBytes);

  for (const file of files) {
    addZipEntry(zipEntries, file.path, encodeText(file.content));
  }

  for (const asset of assets) {
    addZipEntry(zipEntries, asset.zipPath, asset.bytes);
  }

  addZipEntry(zipEntries, manifestFileName, serializeJson(manifest));
  addZipEntry(zipEntries, 'README.md', encodeText(buildReadme(detail, files)));

  return {
    fileName: `design-${detail.framework}.zip`,
    bytes: encodeZip(zipEntries),
    detail,
  };
}
