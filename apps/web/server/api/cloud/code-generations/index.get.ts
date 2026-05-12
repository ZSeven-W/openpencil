import { defineEventHandler, getQuery, createError } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapCodeGeneration } from '../../../utils/cloud-file-mappers';
import { addSignedUrlsToCodegenManifest } from '../../../utils/cloud-codegen-assets';

const querySchema = z.object({
  fileId: z.string().uuid(),
  pageId: z.string().min(1),
  framework: z.string().min(1),
  targetKind: z.enum(['page', 'selection']),
  targetHash: z.string().min(1),
  limit: z.coerce.number().int().positive().max(100).optional().default(20),
});

export default defineEventHandler(async (event) => {
  const parsed = querySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw toApiError(
      400,
      'validation_error',
      'Invalid code generation query',
      parsed.error.flatten(),
    );
  }

  const { supabase, user } = await getCloudSupabase(event);
  const { data, error } = await supabase
    .from('code_generations')
    .select(
      'id,file_id,page_id,framework,target_kind,node_ids,target_hash,document_revision,status,final_code,entry_file,degraded,assets_manifest,model,provider,error,created_at,completed_at,promoted_at',
    )
    .eq('owner_id', user.id)
    .eq('file_id', parsed.data.fileId)
    .eq('page_id', parsed.data.pageId)
    .eq('framework', parsed.data.framework)
    .eq('target_kind', parsed.data.targetKind)
    .eq('target_hash', parsed.data.targetHash)
    .order('created_at', { ascending: false })
    .limit(parsed.data.limit);

  if (error) {
    throw createError({ statusCode: 500, statusMessage: error.message });
  }

  const withSignedUrls = await Promise.all(
    (data ?? []).map(async (row) => ({
      ...row,
      assets_manifest: await addSignedUrlsToCodegenManifest(supabase, row.assets_manifest),
    })),
  );

  return { data: withSignedUrls.map(mapCodeGeneration) };
});
