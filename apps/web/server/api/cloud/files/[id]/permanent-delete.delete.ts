import { defineEventHandler, getRouterParam, createError, setResponseStatus } from 'h3';
import { getCloudSupabase } from '../../../../utils/cloud-supabase';
import { recordCloudActivity } from '../../../../utils/cloud-activity-events';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const file = await supabase
    .from('design_files')
    .select('id')
    .eq('id', id)
    .eq('owner_id', user.id)
    .not('deleted_at', 'is', null)
    .single();
  if (file.error || !file.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }

  await recordCloudActivity({
    supabase,
    ownerId: user.id,
    actorId: user.id,
    fileId: id,
    type: 'file_permanently_deleted',
  });

  const deleted = await supabase
    .from('design_files')
    .delete()
    .eq('id', id)
    .eq('owner_id', user.id);
  if (deleted.error) {
    throw createError({ statusCode: 500, statusMessage: deleted.error.message });
  }

  setResponseStatus(event, 204);
  return null;
});
