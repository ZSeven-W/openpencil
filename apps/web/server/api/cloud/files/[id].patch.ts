import { defineEventHandler, getRouterParam, readBody, createError } from 'h3';
import { randomUUID } from 'node:crypto';
import { z } from 'zod';
import type { PenDocument } from '../../../../src/types/pen';
import { assertCloudFileBodySize } from '../../../utils/cloud-body-limit';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapFileRecord } from '../../../utils/cloud-file-mappers';
import {
  prepareCloudDocumentForStorage,
  resolveCloudDocumentFromStorage,
} from '../../../utils/cloud-document-storage';
import {
  createCloudRevisionConflictDetails,
  isOptimisticUpdateMiss,
  throwCloudRevisionConflict,
} from '../../../utils/cloud-revision-conflict';
import {
  assertCloudFileEditable,
  CLOUD_FILE_SELECT,
  resolveProjectAndFolder,
} from '../../../utils/cloud-file-management';
import { recordCloudActivity } from '../../../utils/cloud-activity-events';
import { pruneAutosaveVersions } from '../../../utils/cloud-version-retention';

const saveFileSchema = z.object({
  name: z.string().trim().min(1).max(160).optional(),
  document: z.record(z.unknown()).optional(),
  expectedRevision: z.number().int().positive().optional(),
  projectId: z.string().uuid().optional().nullable(),
  folderId: z.string().uuid().optional().nullable(),
  metadata: z.record(z.unknown()).optional(),
  starred: z.boolean().optional(),
  source: z
    .enum(['manual_save', 'autosave', 'code_generation', 'restore'])
    .optional()
    .default('manual_save'),
  label: z.string().max(200).optional(),
  snapshot: z.boolean().optional().default(true),
  force: z.boolean().optional().default(false),
}).superRefine((value, ctx) => {
  if (value.document && !value.expectedRevision) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ['expectedRevision'],
      message: 'expectedRevision is required when saving a document',
    });
  }
  if (!value.document && Object.keys(value).length === 0) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: 'At least one cloud file field is required',
    });
  }
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

  const access = await assertCloudFileEditable({
    supabase,
    userId: user.id,
    userEmail: user.email,
    fileId: id,
  });
  const current = await supabase
    .from('design_files')
    .select(CLOUD_FILE_SELECT)
    .eq('id', id)
    .eq('owner_id', access.ownerId)
    .is('deleted_at', null)
    .single();

  if (current.error || !current.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }
  const parsedData = parsed.data;
  const currentData = current.data;

  const updates: Record<string, unknown> = {};
  const isSharedEditor = access.shareRole === 'editor';
  if (isSharedEditor && (parsedData.projectId !== undefined || parsedData.folderId !== undefined)) {
    throw toApiError(403, 'forbidden', 'Shared editors cannot move cloud files');
  }
  if (parsedData.name !== undefined) updates.name = parsedData.name;
  if (parsedData.metadata !== undefined) updates.metadata = parsedData.metadata;
  if (parsedData.starred !== undefined) updates.starred = parsedData.starred;
  if (parsedData.projectId !== undefined || parsedData.folderId !== undefined) {
    const location = await resolveProjectAndFolder({
      supabase,
      userId: access.ownerId,
      projectId: parsedData.projectId ?? currentData.project_id,
      folderId:
        parsedData.folderId !== undefined
          ? parsedData.folderId
          : parsedData.projectId !== undefined
            ? null
            : currentData.folder_id,
    });
    updates.project_id = location.projectId;
    updates.folder_id = location.folderId;
  }

  let document = await resolveCloudDocumentFromStorage(supabase, currentData.document);
  let preparedDocument: Awaited<ReturnType<typeof prepareCloudDocumentForStorage>> | null = null;
  let documentExpectedRevision: number | null = null;
  if (parsedData.document) {
    const expectedRevision = parsedData.expectedRevision;
    if (!expectedRevision) {
      throw toApiError(400, 'validation_error', 'expectedRevision is required');
    }
    documentExpectedRevision = expectedRevision;
    if (Number(currentData.revision) !== expectedRevision && !parsedData.force) {
      throw toApiError(409, 'revision_conflict', 'Cloud file has a newer revision', {
        ...createCloudRevisionConflictDetails(
          id,
          expectedRevision,
          Number(currentData.revision),
        ),
      });
    }

    const nextRevision = Number(currentData.revision) + 1;
    document = parsedData.document as unknown as PenDocument;
    preparedDocument = await prepareCloudDocumentForStorage({
      supabase,
      userId: user.id,
      fileId: id,
      revision: nextRevision,
      document,
      attemptId: randomUUID(),
    });
    updates.document = preparedDocument.storedDocument;
    updates.revision = nextRevision;
  } else if (Object.keys(updates).length === 0) {
    throw toApiError(400, 'validation_error', 'At least one cloud file field is required');
  }

  let updateQuery = supabase
    .from('design_files')
    .update(updates)
    .eq('id', id)
    .eq('owner_id', access.ownerId)
    .is('deleted_at', null);
  if (parsedData.document && !parsedData.force) {
    updateQuery = updateQuery.eq('revision', documentExpectedRevision);
  }

  const { data, error } = await updateQuery.select(CLOUD_FILE_SELECT).single();

  if (isOptimisticUpdateMiss(data, error)) {
    await throwCloudRevisionConflict(
      supabase,
      id,
      access.ownerId,
      documentExpectedRevision ?? Number(currentData.revision),
    );
  }
  if (error || !data) {
    throw createError({ statusCode: 500, statusMessage: error?.message ?? 'Failed to save file' });
  }

  if (parsedData.snapshot) {
    if (!parsedData.document || !preparedDocument) {
      await recordMetadataActivity();
      return { data: mapFileRecord({ ...data, document }) };
    }
    const version = await supabase.from('design_file_versions').insert({
      file_id: data.id,
      owner_id: access.ownerId,
      actor_id: user.id,
      revision: data.revision,
      document: preparedDocument.storedDocument,
      source: parsedData.source,
      label: parsedData.label ?? null,
      size_bytes: new TextEncoder().encode(JSON.stringify(preparedDocument.storedDocument)).byteLength,
    });
    if (version.error) {
      throw createError({ statusCode: 500, statusMessage: version.error.message });
    }
    if (parsedData.source === 'autosave') {
      await pruneAutosaveVersions({
        supabase,
        fileId: data.id,
        ownerId: access.ownerId,
      });
    }
  }

  await recordMetadataActivity();
  if (parsedData.document) {
    await recordCloudActivity({
      supabase,
      ownerId: access.ownerId,
      actorId: user.id,
      fileId: data.id,
      type: parsedData.force ? 'file_force_saved' : 'file_saved',
      metadata: {
        revision: data.revision,
        previousRevision: documentExpectedRevision,
        source: parsedData.source,
        label: parsedData.label ?? null,
        forced: parsedData.force,
      },
    });
  }

  return { data: mapFileRecord({ ...data, document }) };

  async function recordMetadataActivity() {
    if (parsedData.name !== undefined && parsedData.name !== currentData.name) {
      await recordCloudActivity({
        supabase,
        ownerId: access.ownerId,
        actorId: user.id,
        fileId: id,
        type: 'file_renamed',
        metadata: { from: currentData.name, to: parsedData.name },
      });
    }

    const movedProject =
      updates.project_id !== undefined && updates.project_id !== currentData.project_id;
    const movedFolder =
      updates.folder_id !== undefined && updates.folder_id !== currentData.folder_id;
    if (movedProject || movedFolder) {
      await recordCloudActivity({
        supabase,
        ownerId: access.ownerId,
        actorId: user.id,
        fileId: id,
        type: 'file_moved',
        metadata: {
          fromProjectId: currentData.project_id,
          toProjectId: updates.project_id ?? currentData.project_id,
          fromFolderId: currentData.folder_id,
          toFolderId: updates.folder_id ?? currentData.folder_id,
        },
      });
    }
  }
});
