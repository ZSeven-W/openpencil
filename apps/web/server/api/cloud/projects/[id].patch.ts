import { defineEventHandler, getRouterParam, readBody, createError } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapProject } from '../../../utils/cloud-file-mappers';
import { PROJECT_SELECT } from '../../../utils/cloud-file-management';

const updateProjectSchema = z
  .object({
    name: z.string().trim().min(1).max(120).optional(),
    description: z.string().trim().max(500).nullable().optional(),
    icon: z.string().trim().max(80).nullable().optional(),
    color: z.string().trim().max(40).nullable().optional(),
  })
  .refine((value) => Object.keys(value).length > 0, {
    message: 'At least one project field is required',
  });

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing project ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const parsed = updateProjectSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid project payload', parsed.error.flatten());
  }

  const updates: Record<string, string | null> = {};
  if (parsed.data.name !== undefined) updates.name = parsed.data.name;
  if (parsed.data.description !== undefined) updates.description = parsed.data.description;
  if (parsed.data.icon !== undefined) updates.icon = parsed.data.icon;
  if (parsed.data.color !== undefined) updates.color = parsed.data.color;

  const { data, error } = await supabase
    .from('projects')
    .update(updates)
    .eq('id', id)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .select(PROJECT_SELECT)
    .single();

  if (error || !data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud project not found' });
  }

  return { data: mapProject(data) };
});
