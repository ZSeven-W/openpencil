import { defineEventHandler, getRouterParam, readBody, createError, setResponseStatus } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../../utils/cloud-supabase';
import { mapFileRecord } from '../../../../utils/cloud-file-mappers';
import { copyCloudFile } from '../../../../utils/cloud-file-management';

const copyFileSchema = z.object({
  name: z.string().trim().min(1).max(160).optional(),
  projectId: z.string().uuid().optional().nullable(),
  folderId: z.string().uuid().optional().nullable(),
});

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const parsed = copyFileSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid copy payload', parsed.error.flatten());
  }

  const copied = await copyCloudFile({
    supabase,
    userId: user.id,
    fileId: id,
    name: parsed.data.name,
    projectId: parsed.data.projectId,
    folderId: parsed.data.folderId,
  });

  setResponseStatus(event, 201);
  return { data: mapFileRecord({ ...copied.row, document: copied.document }) };
});
