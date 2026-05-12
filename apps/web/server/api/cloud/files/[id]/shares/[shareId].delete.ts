import { defineEventHandler, getRouterParam, createError, setResponseStatus } from 'h3';
import { getCloudSupabase } from '../../../../../utils/cloud-supabase';
import { recordCloudActivity } from '../../../../../utils/cloud-activity-events';

export default defineEventHandler(async (event) => {
  const fileId = getRouterParam(event, 'id');
  const shareId = getRouterParam(event, 'shareId');
  if (!fileId) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });
  if (!shareId) throw createError({ statusCode: 400, statusMessage: 'Missing share ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const revoked = await supabase
    .from('design_file_shares')
    .update({ revoked_at: new Date().toISOString() })
    .eq('id', shareId)
    .eq('file_id', fileId)
    .eq('owner_id', user.id)
    .is('revoked_at', null)
    .select('id')
    .single();

  if (revoked.error || !revoked.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file share not found' });
  }

  await recordCloudActivity({
    supabase,
    ownerId: user.id,
    actorId: user.id,
    fileId,
    type: 'file_share_revoked',
    metadata: { shareId },
  });

  setResponseStatus(event, 204);
  return null;
});
