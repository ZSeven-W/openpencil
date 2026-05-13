import { defineEventHandler, readBody } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { batchCancelCodegenJobs } from '../../../utils/cloud-codegen-jobs';

const batchCancelSchema = z.object({
  jobIds: z.array(z.string().min(1)).min(1).max(50),
});

export default defineEventHandler(async (event) => {
  const parsed = batchCancelSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(
      400,
      'validation_error',
      'Invalid codegen job batch cancel payload',
      parsed.error.flatten(),
    );
  }

  const { supabase, user } = await getCloudSupabase(event);
  return {
    data: await batchCancelCodegenJobs({
      supabase,
      userId: user.id,
      jobIds: parsed.data.jobIds,
    }),
  };
});
