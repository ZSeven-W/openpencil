import { defineEventHandler, getRouterParam, readBody, createError } from 'h3';
import { randomUUID } from 'node:crypto';
import { z } from 'zod';
import type { PenDocument } from '../../../../src/types/pen';
import { assertCloudFileBodySize } from '../../../utils/cloud-body-limit';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapFileRecord } from '../../../utils/cloud-file-mappers';
import { prepareCloudDocumentForStorage } from '../../../utils/cloud-document-storage';
import {
  isOptimisticUpdateMiss,
  throwCloudRevisionConflict,
} from '../../../utils/cloud-revision-conflict';

const saveFileSchema = z.object({
  name: z.string().trim().min(1).max(160).optional(),
  document: z.record(z.unknown()),
  expectedRevision: z.number().int().positive(),
  source: z
    .enum(['manual_save', 'autosave', 'code_generation', 'restore'])
    .optional()
    .default('manual_save'),
  label: z.string().max(200).optional(),
  snapshot: z.boolean().optional().default(true),
});

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });
  const { supabase, user } = await getCloudSupabase(event);
  await assertCloudFileBodySize(event);

  const parsed = saveFileSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid cloud save payload', parsed.error.flatten());
  }

  const current = await supabase
    .from('design_files')
    .select('id,revision')
    .eq('id', id)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .single();

  if (current.error || !current.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }
  if (Number(current.data.revision) !== parsed.data.expectedRevision) {
    throw toApiError(409, 'revision_conflict', 'Cloud file has a newer revision', {
      serverRevision: Number(current.data.revision),
    });
  }

  const nextRevision = parsed.data.expectedRevision + 1;
  const document = parsed.data.document as unknown as PenDocument;
  const preparedDocument = await prepareCloudDocumentForStorage({
    supabase,
    userId: user.id,
    fileId: id,
    revision: nextRevision,
    document,
    attemptId: randomUUID(),
  });

  const { data, error } = await supabase
    .from('design_files')
    .update({
      name: parsed.data.name,
      document: preparedDocument.storedDocument,
      revision: nextRevision,
    })
    .eq('id', id)
    .eq('owner_id', user.id)
    .eq('revision', parsed.data.expectedRevision)
    .select('id,name,document,thumbnail_path,revision,created_at,updated_at')
    .single();

  if (isOptimisticUpdateMiss(data, error)) {
    await throwCloudRevisionConflict(supabase, id, user.id);
  }
  if (error || !data) {
    throw createError({ statusCode: 500, statusMessage: error?.message ?? 'Failed to save file' });
  }

  if (parsed.data.snapshot) {
    const version = await supabase.from('design_file_versions').insert({
      file_id: data.id,
      owner_id: user.id,
      revision: data.revision,
      document: preparedDocument.storedDocument,
      source: parsed.data.source,
      label: parsed.data.label ?? null,
    });
    if (version.error) {
      throw createError({ statusCode: 500, statusMessage: version.error.message });
    }
  }

  return { data: mapFileRecord({ ...data, document }) };
});
