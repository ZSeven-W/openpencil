import { defineEventHandler, getRouterParam, readBody, createError } from 'h3';
import { randomUUID } from 'node:crypto';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../../utils/cloud-supabase';
import { mapFileRecord } from '../../../../utils/cloud-file-mappers';
import {
  prepareCloudDocumentForStorage,
  resolveCloudDocumentFromStorage,
} from '../../../../utils/cloud-document-storage';
import { CLOUD_FILE_SELECT } from '../../../../utils/cloud-file-management';
import { recordCloudActivity } from '../../../../utils/cloud-activity-events';
import { assertCloudFileEditable } from '../../../../utils/cloud-file-management';
import {
  isOptimisticUpdateMiss,
  throwCloudRevisionConflict,
} from '../../../../utils/cloud-revision-conflict';

const restoreSchema = z.object({
  versionId: z.string().uuid(),
});

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  const parsed = restoreSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid restore payload', parsed.error.flatten());
  }

  const { supabase, user } = await getCloudSupabase(event);
  const access = await assertCloudFileEditable({
    supabase,
    userId: user.id,
    userEmail: user.email,
    fileId: id,
  });
  const version = await supabase
    .from('design_file_versions')
    .select('document')
    .eq('id', parsed.data.versionId)
    .eq('file_id', id)
    .eq('owner_id', access.ownerId)
    .single();
  if (version.error || !version.data) {
    throw createError({ statusCode: 404, statusMessage: 'Version not found' });
  }

  const current = await supabase
    .from('design_files')
    .select('revision')
    .eq('id', id)
    .eq('owner_id', access.ownerId)
    .is('deleted_at', null)
    .single();
  if (current.error || !current.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }

  const nextRevision = Number(current.data.revision) + 1;
  const document = await resolveCloudDocumentFromStorage(supabase, version.data.document);
  const preparedDocument = await prepareCloudDocumentForStorage({
    supabase,
    userId: user.id,
    fileId: id,
    revision: nextRevision,
    document,
    attemptId: randomUUID(),
  });

  const updated = await supabase
    .from('design_files')
    .update({ document: preparedDocument.storedDocument, revision: nextRevision })
    .eq('id', id)
    .eq('owner_id', access.ownerId)
    .eq('revision', current.data.revision)
    .select(CLOUD_FILE_SELECT)
    .single();
  if (isOptimisticUpdateMiss(updated.data, updated.error)) {
    await throwCloudRevisionConflict(supabase, id, access.ownerId, Number(current.data.revision));
  }
  if (updated.error || !updated.data) {
    throw createError({
      statusCode: 500,
      statusMessage: updated.error?.message ?? 'Failed to restore version',
    });
  }

  const snapshot = await supabase.from('design_file_versions').insert({
    file_id: id,
    owner_id: access.ownerId,
    actor_id: user.id,
    revision: nextRevision,
    document: preparedDocument.storedDocument,
    source: 'restore',
    label: `Restored ${parsed.data.versionId}`,
    size_bytes: new TextEncoder().encode(JSON.stringify(preparedDocument.storedDocument)).byteLength,
  });
  if (snapshot.error) {
    throw createError({ statusCode: 500, statusMessage: snapshot.error.message });
  }

  await recordCloudActivity({
    supabase,
    ownerId: access.ownerId,
    actorId: user.id,
    fileId: id,
    type: 'file_restored',
    metadata: {
      versionId: parsed.data.versionId,
      revision: nextRevision,
      previousRevision: Number(current.data.revision),
    },
  });

  return { data: mapFileRecord({ ...updated.data, document }) };
});
