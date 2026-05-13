import { defineEventHandler, readBody, setResponseStatus } from 'h3';
import { z } from 'zod';
import type { Framework } from '@zseven-w/pen-types';
import { FRAMEWORKS } from '@zseven-w/pen-types';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { createCodegenJob } from '../../../utils/cloud-codegen-jobs';

const createSchema = z.object({
  jobKind: z.enum(['full_generation', 'patch_generation']).optional(),
  fileId: z.string().uuid(),
  pageId: z.string().min(1),
  framework: z.enum(FRAMEWORKS as [Framework, ...Framework[]]),
  targetKind: z.enum(['page', 'selection']),
  nodeIds: z.array(z.string()),
  targetHash: z.string().min(1),
  documentRevision: z.number().int().positive(),
  model: z.string().min(1),
  provider: z.enum(['anthropic', 'openai', 'gemini']),
  nodes: z.array(z.unknown()),
  variables: z.record(z.string(), z.unknown()).optional(),
  baseGenerationId: z.string().uuid().optional(),
  patchInstruction: z.string().trim().min(1).optional(),
}).superRefine((value, ctx) => {
  if (value.jobKind !== 'patch_generation') return;
  if (!value.baseGenerationId) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ['baseGenerationId'],
      message: 'Patch jobs require baseGenerationId',
    });
  }
  if (!value.patchInstruction) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ['patchInstruction'],
      message: 'Patch jobs require patchInstruction',
    });
  }
});

export default defineEventHandler(async (event) => {
  const parsed = createSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid codegen job payload', parsed.error.flatten());
  }

  const { supabase, user } = await getCloudSupabase(event);
  const job = await createCodegenJob(
    { supabase, userId: user.id, userEmail: user.email },
    parsed.data,
  );
  setResponseStatus(event, 201);
  return { data: job };
});
