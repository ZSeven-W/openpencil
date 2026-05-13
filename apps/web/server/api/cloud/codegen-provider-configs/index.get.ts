import { defineEventHandler, getQuery } from 'h3';
import { z } from 'zod';
import {
  getCloudServiceSupabase,
  getCloudSupabase,
  toApiError,
} from '../../../utils/cloud-supabase';
import { listCodegenProviderConfigs } from '../../../utils/cloud-codegen-jobs';

const querySchema = z.object({
  provider: z.string().min(1).max(64).optional(),
  limit: z.coerce.number().int().positive().max(100).optional().default(50),
  offset: z.coerce.number().int().min(0).optional().default(0),
});

export default defineEventHandler(async (event) => {
  const parsed = querySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid provider config query', parsed.error.flatten());
  }

  await getCloudSupabase(event);
  return {
    data: await listCodegenProviderConfigs({
      supabase: getCloudServiceSupabase(),
      provider: parsed.data.provider,
      limit: parsed.data.limit,
      offset: parsed.data.offset,
    }),
  };
});
