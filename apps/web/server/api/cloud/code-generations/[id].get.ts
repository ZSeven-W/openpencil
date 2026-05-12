import { defineEventHandler, getRouterParam, createError } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { getCodeGenerationDetail } from '../../../utils/cloud-code-generation-records';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing generation ID' });

  const { supabase, user } = await getCloudSupabase(event);
  return {
    data: await getCodeGenerationDetail({
      supabase,
      userId: user.id,
      generationId: id,
    }),
  };
});
