import { defineEventHandler, getRouterParam, createError } from 'h3';
import { getCloudSupabase } from '../../../../utils/cloud-supabase';
import { mapFileRecord } from '../../../../utils/cloud-file-mappers';
import { resolveCloudDocumentFromStorage } from '../../../../utils/cloud-document-storage';
import { CLOUD_FILE_SELECT } from '../../../../utils/cloud-file-management';
import { recordCloudActivity } from '../../../../utils/cloud-activity-events';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const restored = await supabase
    .from('design_files')
    .update({ deleted_at: null })
    .eq('id', id)
    .eq('owner_id', user.id)
    .not('deleted_at', 'is', null)
    .select(CLOUD_FILE_SELECT)
    .single();

  if (restored.error || !restored.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }

  const document = await resolveCloudDocumentFromStorage(supabase, restored.data.document);
  await recordCloudActivity({
    supabase,
    ownerId: user.id,
    actorId: user.id,
    fileId: id,
    type: 'file_restored_from_trash',
  });

  return { data: mapFileRecord({ ...restored.data, document }) };
});
