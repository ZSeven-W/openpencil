import { defineEventHandler, getQuery } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { listTaskNotifications } from '../../../utils/cloud-codegen-jobs';

const querySchema = z.object({
  fileId: z.string().uuid().optional(),
  limit: z.coerce.number().int().positive().max(100).optional().default(10),
  offset: z.coerce.number().int().nonnegative().optional().default(0),
});

export default defineEventHandler(async (event) => {
  const parsed = querySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw toApiError(
      400,
      'validation_error',
      'Invalid task notification query',
      parsed.error.flatten(),
    );
  }

  const { supabase, user } = await getCloudSupabase(event);
  return await listTaskNotifications({
    supabase,
    userId: user.id,
    fileId: parsed.data.fileId,
    limit: parsed.data.limit,
    offset: parsed.data.offset,
  });
});
