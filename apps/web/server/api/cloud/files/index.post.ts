import { randomUUID } from 'node:crypto';
import { defineEventHandler, readBody, createError, setResponseStatus } from 'h3';
import { z } from 'zod';
import type { PenDocument } from '../../../../src/types/pen';
import { assertCloudFileBodySize } from '../../../utils/cloud-body-limit';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapFileRecord } from '../../../utils/cloud-file-mappers';
import { prepareCloudDocumentForStorage } from '../../../utils/cloud-document-storage';

const createFileSchema = z.object({
  name: z.string().trim().min(1).max(160),
  document: z.record(z.unknown()),
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
      name: parsed.data.name,
      document: preparedDocument.storedDocument,
      revision: 1,
    })
    .select('id,name,document,thumbnail_path,revision,created_at,updated_at')
    .single();

  if (error || !data) {
    throw createError({ statusCode: 500, statusMessage: error?.message ?? 'Failed to create file' });
  }

  const versionError = await supabase.from('design_file_versions').insert({
    file_id: data.id,
    owner_id: user.id,
    revision: data.revision,
    document: preparedDocument.storedDocument,
    source: parsed.data.source ?? 'manual_save',
    label: 'Initial version',
  });
  if (versionError.error) {
    throw createError({ statusCode: 500, statusMessage: versionError.error.message });
  }

  setResponseStatus(event, 201);
  return { data: mapFileRecord({ ...data, document }) };
});
