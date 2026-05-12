import { randomUUID } from 'node:crypto';
import { defineEventHandler, readBody, createError, setResponseStatus } from 'h3';
import { z } from 'zod';
import type { PenDocument } from '../../../../src/types/pen';
import { assertCloudFileBodySize } from '../../../utils/cloud-body-limit';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapFileRecord } from '../../../utils/cloud-file-mappers';
import { prepareCloudDocumentForStorage } from '../../../utils/cloud-document-storage';
import {
  CLOUD_FILE_SELECT,
  resolveProjectAndFolder,
} from '../../../utils/cloud-file-management';
import { recordCloudActivity } from '../../../utils/cloud-activity-events';

const createFileSchema = z.object({
  name: z.string().trim().min(1).max(160),
  document: z.record(z.unknown()),
  projectId: z.string().uuid().optional().nullable(),
  folderId: z.string().uuid().optional().nullable(),
  metadata: z.record(z.unknown()).optional(),
  source: z.enum(['import', 'manual_save']).optional(),
});

export default defineEventHandler(async (event) => {
  const { supabase, user } = await getCloudSupabase(event);
  await assertCloudFileBodySize(event);
  const parsed = createFileSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid cloud file payload', parsed.error.flatten());
  }

  const fileId = randomUUID();
  const document = parsed.data.document as unknown as PenDocument;
  const location = await resolveProjectAndFolder({
    supabase,
    userId: user.id,
    projectId: parsed.data.projectId,
    folderId: parsed.data.folderId,
  });
  const preparedDocument = await prepareCloudDocumentForStorage({
    supabase,
    userId: user.id,
    fileId,
    revision: 1,
    document,
    attemptId: randomUUID(),
  });

  const { data, error } = await supabase
    .from('design_files')
    .insert({
      id: fileId,
      owner_id: user.id,
      project_id: location.projectId,
      folder_id: location.folderId,
      name: parsed.data.name,
      document: preparedDocument.storedDocument,
      revision: 1,
      metadata: parsed.data.metadata ?? {},
    })
    .select(CLOUD_FILE_SELECT)
    .single();

  if (error || !data) {
    throw createError({ statusCode: 500, statusMessage: error?.message ?? 'Failed to create file' });
  }

  const versionError = await supabase.from('design_file_versions').insert({
    file_id: data.id,
    owner_id: user.id,
    actor_id: user.id,
    revision: data.revision,
    document: preparedDocument.storedDocument,
    source: parsed.data.source ?? 'manual_save',
    label: 'Initial version',
    size_bytes: new TextEncoder().encode(JSON.stringify(preparedDocument.storedDocument)).byteLength,
  });
  if (versionError.error) {
    throw createError({ statusCode: 500, statusMessage: versionError.error.message });
  }

  await recordCloudActivity({
    supabase,
    ownerId: user.id,
    actorId: user.id,
    fileId: data.id,
    type: 'file_created',
    metadata: {
      name: data.name,
      projectId: data.project_id,
      folderId: data.folder_id,
      revision: data.revision,
    },
  });

  setResponseStatus(event, 201);
  return { data: mapFileRecord({ ...data, document }) };
});
