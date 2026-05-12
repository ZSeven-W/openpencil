import { defineEventHandler, getRouterParam, readBody, createError } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../../../utils/cloud-supabase';
import { mapFileVersion } from '../../../../../utils/cloud-file-mappers';
import { recordCloudActivity } from '../../../../../utils/cloud-activity-events';
import { assertCloudFileEditable } from '../../../../../utils/cloud-file-management';

const VERSION_SELECT = 'id,file_id,revision,source,label,actor_id,size_bytes,created_at';

const labelSchema = z.object({
  label: z.string().trim().max(200).nullable(),
});

export default defineEventHandler(async (event) => {
  const fileId = getRouterParam(event, 'id');
  const versionId = getRouterParam(event, 'versionId');
  if (!fileId) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });
  if (!versionId) throw createError({ statusCode: 400, statusMessage: 'Missing version ID' });

  const parsed = labelSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid version label payload', parsed.error.flatten());
  }

  const { supabase, user } = await getCloudSupabase(event);
  const access = await assertCloudFileEditable({
    supabase,
    userId: user.id,
    userEmail: user.email,
    fileId,
  });

  const updated = await supabase
    .from('design_file_versions')
    .update({ label: parsed.data.label || null })
    .eq('id', versionId)
    .eq('file_id', fileId)
    .eq('owner_id', access.ownerId)
    .select(VERSION_SELECT)
    .single();

  if (updated.error || !updated.data) {
    throw createError({
      statusCode: 404,
      statusMessage: updated.error?.message ?? 'Version not found',
    });
  }

  await recordCloudActivity({
    supabase,
    ownerId: access.ownerId,
    actorId: user.id,
    fileId,
    type: 'version_labeled',
    metadata: {
      versionId,
      revision: updated.data.revision,
      label: updated.data.label,
    },
  });

  return { data: mapFileVersion(updated.data) };
});
