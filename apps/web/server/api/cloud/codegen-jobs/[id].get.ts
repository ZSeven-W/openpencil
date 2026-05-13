import { defineEventHandler, getRouterParam, createError } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { getCodegenJobDetail } from '../../../utils/cloud-codegen-jobs';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing job ID' });

  const { supabase, user } = await getCloudSupabase(event);
  return {
    data: await getCodegenJobDetail({ supabase, userId: user.id, jobId: id }),
  };
});
