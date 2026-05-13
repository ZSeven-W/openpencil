import { defineEventHandler, getRouterParam, createError } from 'h3';
import { getCloudSupabase } from '../../../../utils/cloud-supabase';
import { markTaskNotificationRead } from '../../../../utils/cloud-codegen-jobs';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing notification ID' });

  const { supabase, user } = await getCloudSupabase(event);
  return {
    data: await markTaskNotificationRead({ supabase, userId: user.id, id }),
  };
});
