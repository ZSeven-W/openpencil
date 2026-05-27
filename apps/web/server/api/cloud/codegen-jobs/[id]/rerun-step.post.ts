import { createError, defineEventHandler, getRouterParam, readBody } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../../utils/cloud-supabase';
import { rerunCodegenJobStep } from '../../../../utils/cloud-codegen-jobs';

const rerunStepSchema = z.object({
  stage: z.enum(['quality_check', 'repair', 'chunk']),
  chunkId: z.string().min(1).optional(),
}).superRefine((value, ctx) => {
  if (value.stage === 'chunk' && !value.chunkId) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ['chunkId'],
      message: 'chunkId is required when rerunning a chunk stage',
    });
  }
});

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing job ID' });

  const parsed = rerunStepSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(
      400,
      'validation_error',
      'Invalid codegen job step rerun payload',
      parsed.error.flatten(),
    );
  }

  const { supabase, user } = await getCloudSupabase(event);
  return {
    data: await rerunCodegenJobStep({
      supabase,
      userId: user.id,
      jobId: id,
      stage: parsed.data.stage,
      chunkId: parsed.data.chunkId,
    }),
  };
});
