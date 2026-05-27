import { defineEventHandler, getRouterParam, readBody, createError } from 'h3';
import { randomUUID } from 'node:crypto';
import { z } from 'zod';
import { applyDocumentPatches, type DocumentPatch } from '@zseven-w/pen-core';
import { getCloudSupabase, toApiError } from '../../../../utils/cloud-supabase';
import { prepareCloudDocumentForStorage } from '../../../../utils/cloud-document-storage';
import {
  resolveCloudDocumentWithChanges,
  jsonSizeBytes,
} from '../../../../utils/cloud-document-changes';
import {
  createCloudRevisionConflictDetails,
  isOptimisticUpdateMiss,
  throwCloudRevisionConflict,
} from '../../../../utils/cloud-revision-conflict';
import {
  assertCloudFileEditable,
  CLOUD_FILE_SELECT,
} from '../../../../utils/cloud-file-management';
import { recordCloudActivity } from '../../../../utils/cloud-activity-events';
import { pruneAutosaveVersions } from '../../../../utils/cloud-version-retention';
import { assertCloudFileBodySize } from '../../../../utils/cloud-body-limit';

const PATCH_CHECKPOINT_REVISION_INTERVAL = 20;
const PATCH_CHECKPOINT_MAX_BYTES = 256 * 1024;

const nodePatchSchema = z.object({
  op: z.enum(['add', 'remove', 'modify', 'move']),
  pageId: z.string().nullable(),
  nodeId: z.string().min(1),
  parentId: z.string().nullable().optional(),
  index: z.number().int().nonnegative().optional(),
  fields: z.record(z.string(), z.unknown()).optional(),
  beforeFields: z.record(z.string(), z.unknown()).optional(),
});

const docFieldPatchSchema = z.object({
  op: z.literal('set-doc-field'),
  field: z.enum(['name', 'version', 'variables', 'themes']),
  value: z.unknown().optional(),
  before: z.unknown().optional(),
});

const pagesMetaPatchSchema = z.object({
  op: z.literal('set-pages-meta'),
  pages: z.array(z.object({ id: z.string().min(1), name: z.string() })),
  before: z.array(z.object({ id: z.string().min(1), name: z.string() })).optional(),
});

const documentPatchSchema = z.union([
  nodePatchSchema,
  docFieldPatchSchema,
  pagesMetaPatchSchema,
]);

const patchSaveSchema = z.object({
  baseRevision: z.number().int().positive(),
  patches: z.array(documentPatchSchema),
  clientMutationId: z.string().trim().min(1).max(120),
  name: z.string().trim().min(1).max(160).optional(),
  source: z.enum(['manual_save', 'autosave', 'code_generation']).optional().default('autosave'),
  label: z.string().max(200).optional(),
  snapshot: z.boolean().optional().default(false),
});

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  await assertCloudFileBodySize(event);
  const parsed = patchSaveSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(
      400,
      'validation_error',
      'Invalid document patch payload',
      parsed.error.flatten(),
    );
  }

  const { supabase, user } = await getCloudSupabase(event);
  const access = await assertCloudFileEditable({
    supabase,
    userId: user.id,
    userEmail: user.email,
    fileId: id,
  });
  const input = parsed.data;

  const duplicate = await supabase
    .from('design_file_changes')
    .select('revision')
    .eq('file_id', id)
    .eq('owner_id', access.ownerId)
    .eq('client_mutation_id', input.clientMutationId)
    .limit(1)
    .maybeSingle();
  if (duplicate.error) {
    throw createError({ statusCode: 500, statusMessage: duplicate.error.message });
  }

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

  const currentData = current.data;
  if (duplicate.data) {
    return {
      data: {
        id: currentData.id,
        name: currentData.name,
        revision: Number(duplicate.data.revision),
        updatedAt: currentData.updated_at,
        checkpointRevision: Number(currentData.checkpoint_revision ?? currentData.revision),
        snapshotCreated: false,
      },
    };
  }

  if (Number(currentData.revision) !== input.baseRevision) {
    throw toApiError(409, 'revision_conflict', 'Cloud file has a newer revision', {
      ...createCloudRevisionConflictDetails(id, input.baseRevision, Number(currentData.revision)),
    });
  }

  const nextRevision = Number(currentData.revision) + 1;
  const currentDocument = await resolveCloudDocumentWithChanges(supabase, currentData);
  let nextDocument;
  try {
    nextDocument = applyDocumentPatches(currentDocument, input.patches as DocumentPatch[]);
  } catch (error) {
    throw toApiError(
      400,
      'invalid_document_patch',
      error instanceof Error ? error.message : 'Invalid document patch',
    );
  }

  const patchSizeBytes = jsonSizeBytes(input.patches);
  const shouldSnapshot =
    input.snapshot ||
    input.source !== 'autosave' ||
    patchSizeBytes >= PATCH_CHECKPOINT_MAX_BYTES ||
    nextRevision - Number(currentData.checkpoint_revision ?? currentData.revision) >=
      PATCH_CHECKPOINT_REVISION_INTERVAL;

  const updates: Record<string, unknown> = {
    revision: nextRevision,
  };
  if (input.name !== undefined) updates.name = input.name;

  let preparedDocument: Awaited<ReturnType<typeof prepareCloudDocumentForStorage>> | null = null;
  if (shouldSnapshot) {
    preparedDocument = await prepareCloudDocumentForStorage({
      supabase,
      userId: access.ownerId,
      fileId: id,
      revision: nextRevision,
      document: nextDocument,
      attemptId: randomUUID(),
    });
    updates.document = preparedDocument.storedDocument;
    updates.checkpoint_revision = nextRevision;
    updates.checkpoint_size_bytes = preparedDocument.sizeBytes;
  }

  const updated = await supabase
    .from('design_files')
    .update(updates)
    .eq('id', id)
    .eq('owner_id', access.ownerId)
    .eq('revision', input.baseRevision)
    .is('deleted_at', null)
    .select(CLOUD_FILE_SELECT)
    .single();

  if (isOptimisticUpdateMiss(updated.data, updated.error)) {
    await throwCloudRevisionConflict(supabase, id, access.ownerId, input.baseRevision);
  }
  if (updated.error || !updated.data) {
    throw createError({
      statusCode: 500,
      statusMessage: updated.error?.message ?? 'Failed to save document patches',
    });
  }

  const change = await supabase.from('design_file_changes').insert({
    file_id: id,
    owner_id: access.ownerId,
    actor_id: user.id,
    base_revision: input.baseRevision,
    revision: nextRevision,
    client_mutation_id: input.clientMutationId,
    patches: input.patches,
    source: input.source,
    label: input.label ?? null,
    size_bytes: patchSizeBytes,
  });
  if (change.error) {
    throw createError({ statusCode: 500, statusMessage: change.error.message });
  }

  if (shouldSnapshot && preparedDocument) {
    const version = await supabase.from('design_file_versions').insert({
      file_id: id,
      owner_id: access.ownerId,
      actor_id: user.id,
      revision: nextRevision,
      document: preparedDocument.storedDocument,
      source: input.source,
      label: input.label ?? null,
      size_bytes: preparedDocument.sizeBytes,
    });
    if (version.error) {
      throw createError({ statusCode: 500, statusMessage: version.error.message });
    }
    if (input.source === 'autosave') {
      await pruneAutosaveVersions({ supabase, fileId: id, ownerId: access.ownerId });
    }
  }

  await recordCloudActivity({
    supabase,
    ownerId: access.ownerId,
    actorId: user.id,
    fileId: id,
    type: 'file_saved',
    metadata: {
      revision: nextRevision,
      previousRevision: input.baseRevision,
      source: input.source,
      label: input.label ?? null,
      patchCount: input.patches.length,
      patchSizeBytes,
      snapshot: shouldSnapshot,
    },
  });

  const saved = updated.data;
  return {
    data: {
      id: saved.id,
      name: saved.name,
      revision: Number(saved.revision),
      updatedAt: saved.updated_at,
      checkpointRevision: Number(saved.checkpoint_revision ?? saved.revision),
      snapshotCreated: shouldSnapshot,
    },
  };
});
