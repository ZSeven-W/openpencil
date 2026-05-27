import { defineEventHandler, getQuery } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { listCodegenJobs } from '../../../utils/cloud-codegen-jobs';

const booleanQuery = z.preprocess((value) => {
  if (value === 'true') return true;
  if (value === 'false') return false;
  return value;
}, z.boolean());

const querySchema = z.object({
  fileId: z.string().uuid().optional(),
  status: z.enum(['pending', 'running', 'succeeded', 'failed', 'canceled']).optional(),
  active: booleanQuery.optional(),
  deadLettered: booleanQuery.optional(),
  limit: z.coerce.number().int().positive().max(100).optional().default(10),
  offset: z.coerce.number().int().nonnegative().optional().default(0),
});

export default defineEventHandler(async (event) => {
  const parsed = querySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid codegen job query', parsed.error.flatten());
  }

  const { supabase, user } = await getCloudSupabase(event);
  return await listCodegenJobs({
    supabase,
    userId: user.id,
    fileId: parsed.data.fileId,
    status: parsed.data.status,
    active: parsed.data.active,
    deadLettered: parsed.data.deadLettered,
    limit: parsed.data.limit,
    offset: parsed.data.offset,
  });
});
