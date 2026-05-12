import { defineEventHandler, getRouterParam, createError } from 'h3';
import { getCloudSupabase } from '../../../../../utils/cloud-supabase';
import { listOwnedFileShares } from '../../../../../utils/cloud-file-management';
import { mapFileShare } from '../../../../../utils/cloud-file-mappers';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const shares = await listOwnedFileShares({ supabase, userId: user.id, fileId: id });
  return { data: shares.map(mapFileShare) };
});
