import { defineEventHandler, getRouterParam, createError, setResponseStatus } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { recordCloudActivity } from '../../../utils/cloud-activity-events';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const { data, error } = await supabase
    .from('design_files')
    .update({ deleted_at: new Date().toISOString() })
    .eq('id', id)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .select('id')
    .single();

  if (error || !data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }

  await recordCloudActivity({
    supabase,
    ownerId: user.id,
    actorId: user.id,
    fileId: id,
    type: 'file_deleted',
  });

  setResponseStatus(event, 204);
  return null;
});
