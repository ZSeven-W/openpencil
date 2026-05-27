import { createError, defineEventHandler, getRouterParam, setResponseStatus } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { deleteTaskNotification } from '../../../utils/cloud-codegen-jobs';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing notification ID' });

  const { supabase, user } = await getCloudSupabase(event);
  await deleteTaskNotification({ supabase, userId: user.id, id });
  setResponseStatus(event, 204);
  return null;
});
