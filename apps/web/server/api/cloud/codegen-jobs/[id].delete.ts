import { defineEventHandler, getRouterParam, createError, setResponseStatus } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { deleteCodegenJob } from '../../../utils/cloud-codegen-jobs';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing job ID' });

  const { supabase, user } = await getCloudSupabase(event);
  await deleteCodegenJob({ supabase, userId: user.id, jobId: id });
  setResponseStatus(event, 204);
  return null;
});
