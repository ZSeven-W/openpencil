import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';
import type {
  CloudCodeGeneration,
  CloudCodeGenerationDetail,
  CloudCodegenChunk,
  CodegenAssetManifestEntry,
  SaveCodegenFileInput,
  CodegenTargetKind,
  SaveCodegenAssetInput,
} from '../../src/types/cloud';
import type { Framework } from '@zseven-w/pen-types';
import { mapCodeGeneration, mapCodeGenerationDetail } from './cloud-file-mappers';
import {
  addSignedUrlsToCodegenManifest,
  persistCodeGenerationAssets,
} from './cloud-codegen-assets';
import { CLOUD_DOCUMENT_STORAGE_BUCKET } from './cloud-document-storage';
import { recordCloudActivity } from './cloud-activity-events';

const CODE_GENERATION_SELECT =
  'id,file_id,page_id,framework,target_kind,node_ids,target_hash,document_revision,status,final_code,entry_file,degraded,assets_manifest,model,provider,error,created_at,completed_at,promoted_at';

interface CodeGenerationRecordRow {
  id: string;
  file_id: string;
  page_id: string;
  framework: Framework;
  target_kind: CodegenTargetKind;
  node_ids: string[];
  target_hash: string;
  document_revision: number;
  status: 'done' | 'degraded' | 'failed';
  final_code: string | null;
  entry_file: string | null;
  degraded: boolean;
  assets_manifest: CodegenAssetManifestEntry[];
  model: string | null;
  provider: string | null;
  error: string | null;
  created_at: string;
  completed_at: string | null;
  promoted_at: string | null;
}

export interface CreateCodeGenerationRecordInput {
  supabase: SupabaseClient;
  userId: string;
  fileId: string;
  pageId: string;
  framework: Framework;
  targetKind: CodegenTargetKind;
  nodeIds: string[];
  targetHash: string;
  documentRevision: number;
  status: 'done' | 'degraded' | 'failed';
  finalCode?: string;
  entryFile?: string;
  degraded: boolean;
  assets: SaveCodegenAssetInput[];
  files: SaveCodegenFileInput[];
  model?: string;
  provider?: string;
  error?: string;
  chunks: CloudCodegenChunk[];
}

export interface GetCodeGenerationDetailInput {
  supabase: SupabaseClient;
  userId: string;
  generationId: string;
}

export interface PromoteCodeGenerationRecordInput {
  supabase: SupabaseClient;
  userId: string;
  generationId: string;
}

export interface DeleteCodeGenerationRecordInput {
  supabase: SupabaseClient;
  userId: string;
  generationId: string;
}

async function loadCodeGenerationChunks(input: { supabase: SupabaseClient; generationId: string }) {
  const chunkRows = await input.supabase
    .from('code_generation_chunks')
    .select('chunk_id,name,status,code,contract,error,order_index')
    .eq('generation_id', input.generationId)
    .order('order_index', { ascending: true });
  if (chunkRows.error) {
    throw createError({ statusCode: 500, statusMessage: chunkRows.error.message });
  }
  return chunkRows.data ?? [];
}

async function loadCodeGenerationRecord(input: {
  supabase: SupabaseClient;
  userId: string;
  generationId: string;
}): Promise<CodeGenerationRecordRow> {
  const generation = await input.supabase
    .from('code_generations')
    .select(CODE_GENERATION_SELECT)
    .eq('id', input.generationId)
    .eq('owner_id', input.userId)
    .single();

  if (generation.error || !generation.data) {
    throw createError({ statusCode: 404, statusMessage: 'Code generation not found' });
  }

  return generation.data;
}

export async function loadCodeGenerationFiles(input: {
  supabase: SupabaseClient;
  generationId: string;
}) {
  const fileRows = await input.supabase
    .from('code_generation_files')
    .select('id,generation_id,path,language,role,content,size_bytes,order_index,created_at')
    .eq('generation_id', input.generationId)
    .order('order_index', { ascending: true });
  if (fileRows.error) {
    throw createError({ statusCode: 500, statusMessage: fileRows.error.message });
  }
  return fileRows.data ?? [];
}

export async function getCodeGenerationDetail(
  input: GetCodeGenerationDetailInput,
): Promise<CloudCodeGenerationDetail> {
  const generation = await loadCodeGenerationRecord(input);

  const [chunks, files] = await Promise.all([
    loadCodeGenerationChunks({ supabase: input.supabase, generationId: input.generationId }),
    loadCodeGenerationFiles({ supabase: input.supabase, generationId: input.generationId }),
  ]);

  return mapCodeGenerationDetail({
    generation: {
      ...generation,
      assets_manifest: await addSignedUrlsToCodegenManifest(
        input.supabase,
        generation.assets_manifest,
      ),
    },
    files,
    chunks,
  });
}

export async function promoteCodeGenerationRecord(
  input: PromoteCodeGenerationRecordInput,
): Promise<CloudCodeGeneration> {
  const generation = await loadCodeGenerationRecord(input);
  const promotedAt = new Date().toISOString();

  const cleared = await input.supabase
    .from('code_generations')
    .update({ promoted_at: null })
    .eq('owner_id', input.userId)
    .eq('file_id', generation.file_id)
    .eq('page_id', generation.page_id)
    .eq('framework', generation.framework)
    .eq('target_kind', generation.target_kind)
    .eq('target_hash', generation.target_hash)
    .neq('id', input.generationId);
  if (cleared.error) {
    throw createError({
      statusCode: 500,
      statusMessage: cleared.error.message ?? 'Failed to clear promoted generation',
    });
  }

  const updated = await input.supabase
    .from('code_generations')
    .update({ promoted_at: promotedAt })
    .eq('id', input.generationId)
    .eq('owner_id', input.userId)
    .select(CODE_GENERATION_SELECT)
    .single();

  if (updated.error || !updated.data) {
    throw createError({
      statusCode: 500,
      statusMessage: updated.error?.message ?? 'Failed to promote code generation',
    });
  }

  await recordCloudActivity({
    supabase: input.supabase,
    ownerId: input.userId,
    actorId: input.userId,
    fileId: generation.file_id,
    generationId: input.generationId,
    type: 'codegen_promoted',
    metadata: {
      framework: generation.framework,
      pageId: generation.page_id,
      targetKind: generation.target_kind,
      targetHash: generation.target_hash,
    },
  });

  return mapCodeGeneration({
    ...updated.data,
    assets_manifest: await addSignedUrlsToCodegenManifest(
      input.supabase,
      updated.data.assets_manifest,
    ),
  });
}

async function removeCodeGenerationAssets(input: {
  supabase: SupabaseClient;
  userId: string;
  fileId: string;
  assetsManifest: CodegenAssetManifestEntry[];
}) {
  const storagePaths = Array.from(
    new Set(input.assetsManifest.map((entry) => entry.storagePath).filter(Boolean) as string[]),
  );
  if (storagePaths.length === 0) return;

  const storageRemoval = await input.supabase.storage
    .from(CLOUD_DOCUMENT_STORAGE_BUCKET)
    .remove(storagePaths);
  if (storageRemoval.error) {
    throw createError({
      statusCode: 500,
      statusMessage: storageRemoval.error.message ?? 'Failed to remove code generation assets',
    });
  }

  const assetRows = await input.supabase
    .from('file_assets')
    .delete()
    .eq('file_id', input.fileId)
    .eq('owner_id', input.userId)
    .eq('kind', 'codegen_asset')
    .in('path', storagePaths);
  if (assetRows.error) {
    throw createError({
      statusCode: 500,
      statusMessage: assetRows.error.message ?? 'Failed to remove code generation asset records',
    });
  }
}

export async function deleteCodeGenerationRecord(
  input: DeleteCodeGenerationRecordInput,
): Promise<void> {
  const generation = await loadCodeGenerationRecord(input);

  await removeCodeGenerationAssets({
    supabase: input.supabase,
    userId: input.userId,
    fileId: generation.file_id,
    assetsManifest: generation.assets_manifest ?? [],
  });

  await recordCloudActivity({
    supabase: input.supabase,
    ownerId: input.userId,
    actorId: input.userId,
    fileId: generation.file_id,
    generationId: input.generationId,
    type: 'codegen_deleted',
    metadata: {
      framework: generation.framework,
      pageId: generation.page_id,
      targetKind: generation.target_kind,
      targetHash: generation.target_hash,
    },
  });

  const deleted = await input.supabase
    .from('code_generations')
    .delete()
    .eq('id', input.generationId)
    .eq('owner_id', input.userId);

  if (deleted.error) {
    throw createError({
      statusCode: 500,
      statusMessage: deleted.error.message ?? 'Failed to delete code generation',
    });
  }
}

export async function createCodeGenerationRecord(
  input: CreateCodeGenerationRecordInput,
): Promise<CloudCodeGenerationDetail> {
  const generation = await input.supabase
    .from('code_generations')
    .insert({
      file_id: input.fileId,
      owner_id: input.userId,
      page_id: input.pageId,
      framework: input.framework,
      target_kind: input.targetKind,
      node_ids: input.nodeIds,
      target_hash: input.targetHash,
      document_revision: input.documentRevision,
      status: input.status,
      final_code: input.finalCode ?? null,
      entry_file: input.entryFile ?? input.files[0]?.path ?? null,
      degraded: input.degraded,
      assets_manifest: [],
      model: input.model ?? null,
      provider: input.provider ?? null,
      error: input.error ?? null,
      completed_at: new Date().toISOString(),
    })
    .select(CODE_GENERATION_SELECT)
    .single();

  if (generation.error || !generation.data) {
    throw createError({
      statusCode: 500,
      statusMessage: generation.error?.message ?? 'Failed to save code generation',
    });
  }

  let generationRow = generation.data;
  if (input.assets.length > 0) {
    const assetsManifest = await persistCodeGenerationAssets({
      supabase: input.supabase,
      userId: input.userId,
      fileId: input.fileId,
      generationId: generation.data.id,
      assets: input.assets,
    });

    const updated = await input.supabase
      .from('code_generations')
      .update({ assets_manifest: assetsManifest })
      .eq('id', generation.data.id)
      .eq('owner_id', input.userId)
      .select(CODE_GENERATION_SELECT)
      .single();
    if (updated.error || !updated.data) {
      throw createError({
        statusCode: 500,
        statusMessage: updated.error?.message ?? 'Failed to update generation assets',
      });
    }
    generationRow = updated.data;
  }

  if (input.chunks.length > 0) {
    const chunks = await input.supabase.from('code_generation_chunks').insert(
      input.chunks.map((chunk, index) => ({
        generation_id: generation.data.id,
        chunk_id: chunk.chunkId,
        name: chunk.name,
        status: chunk.status,
        code: chunk.code ?? null,
        contract: chunk.contract ?? null,
        error: chunk.error ?? null,
        order_index: chunk.orderIndex ?? index,
      })),
    );
    if (chunks.error) {
      throw createError({ statusCode: 500, statusMessage: chunks.error.message });
    }
  }

  if (input.files.length > 0) {
    const files = await input.supabase.from('code_generation_files').insert(
      input.files.map((file, index) => ({
        generation_id: generation.data.id,
        path: file.path,
        language: file.language,
        role: file.role,
        content: file.content,
        size_bytes: new TextEncoder().encode(file.content).byteLength,
        order_index: file.orderIndex ?? index,
      })),
    );
    if (files.error) {
      throw createError({ statusCode: 500, statusMessage: files.error.message });
    }
  }

  const [chunks, files] = await Promise.all([
    loadCodeGenerationChunks({ supabase: input.supabase, generationId: generation.data.id }),
    loadCodeGenerationFiles({ supabase: input.supabase, generationId: generation.data.id }),
  ]);

  await recordCloudActivity({
    supabase: input.supabase,
    ownerId: input.userId,
    actorId: input.userId,
    fileId: input.fileId,
    generationId: generation.data.id,
    type: 'codegen_created',
    metadata: {
      framework: input.framework,
      pageId: input.pageId,
      targetKind: input.targetKind,
      targetHash: input.targetHash,
      documentRevision: input.documentRevision,
      status: input.status,
      fileCount: files.length,
    },
  });

  return mapCodeGenerationDetail({
    generation: {
      ...generationRow,
      assets_manifest: await addSignedUrlsToCodegenManifest(
        input.supabase,
        generationRow.assets_manifest,
      ),
    },
    files,
    chunks,
  });
}
