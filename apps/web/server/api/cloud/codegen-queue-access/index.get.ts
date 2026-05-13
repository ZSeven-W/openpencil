import { defineEventHandler } from 'h3';
import { getCloudServiceSupabase, getCloudSupabase } from '../../../utils/cloud-supabase';
import { getCodegenQueueAccess } from '../../../utils/cloud-codegen-jobs';

export default defineEventHandler(async (event) => {
  const { user } = await getCloudSupabase(event);
  return {
    data: await getCodegenQueueAccess({
      supabase: getCloudServiceSupabase(),
      userId: user.id,
      userEmail: user.email,
    }),
  };
});
