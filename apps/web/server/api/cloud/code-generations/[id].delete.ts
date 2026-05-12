import { defineEventHandler, getRouterParam, createError, setResponseStatus } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { deleteCodeGenerationRecord } from '../../../utils/cloud-code-generation-records';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing generation ID' });

  const { supabase, user } = await getCloudSupabase(event);
  await deleteCodeGenerationRecord({
    supabase,
    userId: user.id,
    generationId: id,
  });

  setResponseStatus(event, 204);
  return null;
});
