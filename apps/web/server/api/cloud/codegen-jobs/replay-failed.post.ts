import { defineEventHandler, readBody } from 'h3';
import { z } from 'zod';
import {
  getCloudServiceSupabase,
  getCloudSupabase,
  toApiError,
} from '../../../utils/cloud-supabase';
import { replayFailedCodegenJobs } from '../../../utils/cloud-codegen-jobs';

const replayFailedSchema = z.object({
  jobIds: z.array(z.string().min(1)).max(50).optional(),
  provider: z.string().min(1).max(64).optional(),
  failureType: z
    .enum(['rate_limit', 'timeout', 'provider_error', 'canceled', 'execution_error'])
    .optional(),
  deadLetteredFrom: z.string().datetime().optional(),
  deadLetteredTo: z.string().datetime().optional(),
  limit: z.number().int().positive().max(50).optional().default(50),
});

export default defineEventHandler(async (event) => {
  const parsed = replayFailedSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(
      400,
      'validation_error',
      'Invalid codegen job replay payload',
      parsed.error.flatten(),
    );
  }

  const { supabase, user } = await getCloudSupabase(event);
  return {
    data: await replayFailedCodegenJobs({
      supabase,
      adminSupabase: getCloudServiceSupabase(),
      userId: user.id,
      userEmail: user.email,
      jobIds: parsed.data.jobIds,
      provider: parsed.data.provider,
      failureType: parsed.data.failureType,
      deadLetteredFrom: parsed.data.deadLetteredFrom,
      deadLetteredTo: parsed.data.deadLetteredTo,
      limit: parsed.data.limit,
    }),
  };
});
