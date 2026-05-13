import { defineEventHandler, getQuery } from 'h3';
import { z } from 'zod';
import { getCloudServiceSupabase, getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { getCodegenWorkerOverview } from '../../../utils/cloud-codegen-jobs';

const querySchema = z.object({
  workerLimit: z.coerce.number().int().positive().max(100).optional().default(50),
  workerOffset: z.coerce.number().int().min(0).optional().default(0),
  providerLimit: z.coerce.number().int().positive().max(100).optional().default(50),
  providerOffset: z.coerce.number().int().min(0).optional().default(0),
  failedLimit: z.coerce.number().int().positive().max(100).optional().default(50),
  failedOffset: z.coerce.number().int().min(0).optional().default(0),
  provider: z.string().min(1).max(64).optional(),
  failureType: z
    .enum(['rate_limit', 'timeout', 'provider_error', 'canceled', 'execution_error'])
    .optional(),
  deadLetteredFrom: z.string().datetime().optional(),
  deadLetteredTo: z.string().datetime().optional(),
});

export default defineEventHandler(async (event) => {
  const parsed = querySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid worker overview query', parsed.error.flatten());
  }
  const { supabase, user } = await getCloudSupabase(event);
  return {
    data: await getCodegenWorkerOverview({
      supabase,
      adminSupabase: getCloudServiceSupabase(),
      userId: user.id,
      ...parsed.data,
    }),
  };
});
