import { defineEventHandler, getQuery } from 'h3';
import { z } from 'zod';
import {
  getCloudServiceSupabase,
  getCloudSupabase,
  toApiError,
} from '../../../utils/cloud-supabase';
import { getCodegenWorkerRuntimeStats } from '../../../utils/cloud-codegen-jobs';

const querySchema = z.object({
  days: z.coerce.number().int().min(1).max(30).optional().default(7),
});

export default defineEventHandler(async (event) => {
  const parsed = querySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid worker stats query', parsed.error.flatten());
  }

  const { supabase, user } = await getCloudSupabase(event);
  return {
    data: await getCodegenWorkerRuntimeStats({
      supabase,
      adminSupabase: getCloudServiceSupabase(),
      userId: user.id,
      days: parsed.data.days,
    }),
  };
});
