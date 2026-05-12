import { defineEventHandler, getQuery, createError } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapFolder } from '../../../utils/cloud-file-mappers';
import {
  assertProjectAccess,
  FOLDER_SELECT,
  getDefaultProject,
} from '../../../utils/cloud-file-management';

const querySchema = z.object({
  projectId: z.string().uuid().optional(),
  parentId: z.preprocess(
    (value) => (value === 'null' ? null : value),
    z.string().uuid().nullable().optional(),
  ),
});

export default defineEventHandler(async (event) => {
  const parsed = querySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid folder query', parsed.error.flatten());
  }

  const { supabase, user } = await getCloudSupabase(event);
  const project = parsed.data.projectId
    ? (await assertProjectAccess({
        supabase,
        userId: user.id,
        projectId: parsed.data.projectId,
      }),
      { id: parsed.data.projectId })
    : await getDefaultProject({ supabase, userId: user.id });

  let query = supabase
    .from('folders')
    .select(FOLDER_SELECT)
    .eq('owner_id', user.id)
    .eq('project_id', project.id)
    .is('deleted_at', null);

  if (parsed.data.parentId !== undefined) {
    query = parsed.data.parentId === null
      ? query.is('parent_id', null)
      : query.eq('parent_id', parsed.data.parentId);
  }

  const { data, error } = await query.order('sort_order', { ascending: true }).order('name', {
    ascending: true,
  });
  if (error) throw createError({ statusCode: 500, statusMessage: error.message });

  return { data: (data ?? []).map(mapFolder) };
});
