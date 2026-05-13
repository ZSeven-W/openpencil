import { createError, defineEventHandler, getRouterParam, readBody } from 'h3';
import { z } from 'zod';
import {
  getCloudServiceSupabase,
  getCloudSupabase,
  toApiError,
} from '../../../utils/cloud-supabase';
import { updateCodegenProviderConfig } from '../../../utils/cloud-codegen-jobs';

const providerConfigSchema = z.object({
  enabled: z.boolean(),
  maxPerMinute: z.number().int().min(1).max(10_000),
  circuitThreshold: z.number().int().min(1).max(1_000),
  circuitOpenMs: z.number().int().min(1_000).max(86_400_000),
  reason: z.string().max(500).optional(),
});

export default defineEventHandler(async (event) => {
  const provider = getRouterParam(event, 'provider');
  if (!provider) {
    throw createError({ statusCode: 400, statusMessage: 'Missing provider' });
  }

  const parsed = providerConfigSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(
      400,
      'validation_error',
      'Invalid provider config payload',
      parsed.error.flatten(),
    );
  }

  const { user } = await getCloudSupabase(event);
  return {
    data: await updateCodegenProviderConfig({
      supabase: getCloudServiceSupabase(),
      actorId: user.id,
      actorEmail: user.email,
      provider,
      ...parsed.data,
    }),
  };
});
