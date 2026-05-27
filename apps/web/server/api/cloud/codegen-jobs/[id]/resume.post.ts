import { createError, defineEventHandler, getRouterParam, readBody } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../../utils/cloud-supabase';
import { resumeCodegenJob } from '../../../../utils/cloud-codegen-jobs';

const resumeSchema = z
  .object({
    stage: z.enum(['from_failed_stage', 'quality_check', 'repair', 'chunk']).optional(),
  })
  .optional()
  .default({});

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing job ID' });

  const parsed = resumeSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(
      400,
      'validation_error',
      'Invalid codegen job resume payload',
      parsed.error.flatten(),
    );
  }

  const { supabase, user } = await getCloudSupabase(event);
  return {
    data: await resumeCodegenJob({
      supabase,
      userId: user.id,
      jobId: id,
      stage: parsed.data.stage,
    }),
  };
});
