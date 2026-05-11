import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';
import type {
  CloudCodeGenerationDetail,
  CloudCodegenChunk,
  CodegenTargetKind,
  SaveCodegenAssetInput,
} from '../../src/types/cloud';
import type { Framework } from '@zseven-w/pen-types';
import { mapCodeGenerationDetail } from './cloud-file-mappers';
import {
  addSignedUrlsToCodegenManifest,
  persistCodeGenerationAssets,
} from './cloud-codegen-assets';

const CODE_GENERATION_SELECT =
  'id,file_id,page_id,framework,target_kind,node_ids,target_hash,document_revision,status,final_code,degraded,assets_manifest,model,provider,error,created_at,completed_at';

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
  degraded: boolean;
  assets: SaveCodegenAssetInput[];
  model?: string;
  provider?: string;
  error?: string;
  chunks: CloudCodegenChunk[];
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

  const chunkRows = await input.supabase
    .from('code_generation_chunks')
    .select('chunk_id,name,status,code,contract,error,order_index')
    .eq('generation_id', generation.data.id)
    .order('order_index', { ascending: true });
  if (chunkRows.error) {
    throw createError({ statusCode: 500, statusMessage: chunkRows.error.message });
  }

  return mapCodeGenerationDetail({
    generation: {
      ...generationRow,
      assets_manifest: await addSignedUrlsToCodegenManifest(
        input.supabase,
        generationRow.assets_manifest,
      ),
    },
    chunks: chunkRows.data ?? [],
  });
}
