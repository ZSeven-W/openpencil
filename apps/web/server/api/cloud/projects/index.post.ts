import { defineEventHandler, readBody, createError, setResponseStatus } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapProject } from '../../../utils/cloud-file-mappers';
import { PROJECT_SELECT } from '../../../utils/cloud-file-management';

const createProjectSchema = z.object({
  name: z.string().trim().min(1).max(120),
  description: z.string().trim().max(500).nullable().optional(),
  icon: z.string().trim().max(80).nullable().optional(),
  color: z.string().trim().max(40).nullable().optional(),
});

export default defineEventHandler(async (event) => {
  const { supabase, user } = await getCloudSupabase(event);
  const parsed = createProjectSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid project payload', parsed.error.flatten());
  }

  const { data, error } = await supabase
    .from('projects')
    .insert({
      owner_id: user.id,
      name: parsed.data.name,
      description: parsed.data.description ?? null,
      icon: parsed.data.icon ?? null,
      color: parsed.data.color ?? null,
    })
    .select(PROJECT_SELECT)
    .single();

  if (error || !data) {
    throw createError({
      statusCode: 500,
      statusMessage: error?.message ?? 'Failed to create project',
    });
  }

  setResponseStatus(event, 201);
  return { data: mapProject(data) };
});
