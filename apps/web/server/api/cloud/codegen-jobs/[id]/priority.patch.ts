import { createError, defineEventHandler, getRouterParam, readBody } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../../utils/cloud-supabase';
import { updateCodegenJobPriority } from '../../../../utils/cloud-codegen-jobs';

const prioritySchema = z.object({
  priority: z.number().int().min(-1000).max(1000),
});

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing job ID' });

  const parsed = prioritySchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(
      400,
      'validation_error',
      'Invalid codegen job priority payload',
      parsed.error.flatten(),
    );
  }

  const { supabase, user } = await getCloudSupabase(event);
  return {
    data: await updateCodegenJobPriority({
      supabase,
      userId: user.id,
      jobId: id,
      priority: parsed.data.priority,
    }),
  };
});
