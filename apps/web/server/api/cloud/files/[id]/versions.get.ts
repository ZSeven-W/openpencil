import { defineEventHandler, getQuery, getRouterParam, createError } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../../utils/cloud-supabase';
import { mapFileVersion } from '../../../../utils/cloud-file-mappers';
import { assertCloudFileReadable } from '../../../../utils/cloud-file-management';

const versionQuerySchema = z.object({
  limit: z.coerce.number().int().positive().max(100).optional().default(10),
  offset: z.coerce.number().int().nonnegative().optional().default(0),
});

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });
  const parsed = versionQuerySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid version query', parsed.error.flatten());
  }

  const { supabase, user } = await getCloudSupabase(event);
  const access = await assertCloudFileReadable({
    supabase,
    userId: user.id,
    userEmail: user.email,
    fileId: id,
  });
  const { data, error, count } = await supabase
    .from('design_file_versions')
    .select('id,file_id,revision,source,label,actor_id,size_bytes,created_at', { count: 'exact' })
    .eq('file_id', id)
    .eq('owner_id', access.ownerId)
    .order('revision', { ascending: false })
    .range(parsed.data.offset, parsed.data.offset + parsed.data.limit - 1);

  if (error) {
    throw createError({ statusCode: 500, statusMessage: error.message });
  }

  return {
    data: (data ?? []).map(mapFileVersion),
    page: {
      total: count ?? 0,
      limit: parsed.data.limit,
      offset: parsed.data.offset,
    },
  };
});
