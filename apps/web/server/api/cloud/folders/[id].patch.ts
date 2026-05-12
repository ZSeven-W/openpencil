import { defineEventHandler, getRouterParam, readBody, createError } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapFolder } from '../../../utils/cloud-file-mappers';
import {
  assertFolderAccess,
  FOLDER_SELECT,
  resolveProjectAndFolder,
} from '../../../utils/cloud-file-management';

const updateFolderSchema = z
  .object({
    projectId: z.string().uuid().optional().nullable(),
    parentId: z.string().uuid().optional().nullable(),
    name: z.string().trim().min(1).max(120).optional(),
    sortOrder: z.number().int().optional(),
  })
  .refine((value) => Object.keys(value).length > 0, {
    message: 'At least one folder field is required',
  });

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing folder ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const parsed = updateFolderSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid folder payload', parsed.error.flatten());
  }
  if (parsed.data.parentId === id) {
    throw toApiError(400, 'validation_error', 'Folder cannot be its own parent');
  }

  const current = await supabase
    .from('folders')
    .select(FOLDER_SELECT)
    .eq('id', id)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .single();
  if (current.error || !current.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud folder not found' });
  }

  const location = await resolveProjectAndFolder({
    supabase,
    userId: user.id,
    projectId: parsed.data.projectId ?? current.data.project_id,
  });
  if (parsed.data.parentId) {
    await assertFolderAccess({
      supabase,
      userId: user.id,
      projectId: location.projectId,
      folderId: parsed.data.parentId,
    });
  }

  const updates: Record<string, string | number | null> = {};
  if (parsed.data.projectId !== undefined) updates.project_id = location.projectId;
  if (parsed.data.parentId !== undefined) updates.parent_id = parsed.data.parentId;
  if (parsed.data.name !== undefined) updates.name = parsed.data.name;
  if (parsed.data.sortOrder !== undefined) updates.sort_order = parsed.data.sortOrder;

  const { data, error } = await supabase
    .from('folders')
    .update(updates)
    .eq('id', id)
    .eq('owner_id', user.id)
    .select(FOLDER_SELECT)
    .single();

  if (error || !data) {
    throw createError({
      statusCode: 500,
      statusMessage: error?.message ?? 'Failed to update folder',
    });
  }

  return { data: mapFolder(data) };
});
