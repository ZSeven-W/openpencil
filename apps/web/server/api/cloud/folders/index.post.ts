import { defineEventHandler, readBody, createError, setResponseStatus } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapFolder } from '../../../utils/cloud-file-mappers';
import {
  assertFolderAccess,
  FOLDER_SELECT,
  resolveProjectAndFolder,
} from '../../../utils/cloud-file-management';

const createFolderSchema = z.object({
  projectId: z.string().uuid().optional().nullable(),
  parentId: z.string().uuid().optional().nullable(),
  name: z.string().trim().min(1).max(120),
  sortOrder: z.number().int().optional(),
});

export default defineEventHandler(async (event) => {
  const { supabase, user } = await getCloudSupabase(event);
  const parsed = createFolderSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid folder payload', parsed.error.flatten());
  }

  const location = await resolveProjectAndFolder({
    supabase,
    userId: user.id,
    projectId: parsed.data.projectId,
  });
  if (parsed.data.parentId) {
    await assertFolderAccess({
      supabase,
      userId: user.id,
      projectId: location.projectId,
      folderId: parsed.data.parentId,
    });
  }

  const { data, error } = await supabase
    .from('folders')
    .insert({
      owner_id: user.id,
      project_id: location.projectId,
      parent_id: parsed.data.parentId ?? null,
      name: parsed.data.name,
      sort_order: parsed.data.sortOrder ?? 0,
    })
    .select(FOLDER_SELECT)
    .single();

  if (error || !data) {
    throw createError({
      statusCode: 500,
      statusMessage: error?.message ?? 'Failed to create folder',
    });
  }

  setResponseStatus(event, 201);
  return { data: mapFolder(data) };
});
