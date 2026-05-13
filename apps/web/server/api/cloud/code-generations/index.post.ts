import { defineEventHandler, readBody, createError, setResponseStatus } from 'h3';
import { z } from 'zod';
import type { Framework } from '@zseven-w/pen-types';
import { FRAMEWORKS } from '@zseven-w/pen-types';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { createCodeGenerationRecord } from '../../../utils/cloud-code-generation-records';

const assetSchema = z.object({
  id: z.string(),
  relativePath: z.string(),
  zipPath: z.string(),
  mimeType: z.string(),
  bytesBase64: z.string(),
  sourceNodeId: z.string(),
  sourceNodeName: z.string().optional(),
  sourceKind: z.enum(['image-node', 'image-fill']),
});

const chunkSchema = z.object({
  chunkId: z.string(),
  name: z.string(),
  status: z.string(),
  code: z.string().optional(),
  contract: z.unknown().optional(),
  error: z.string().optional(),
  orderIndex: z.number().int().optional(),
});

const fileSchema = z.object({
  path: z
    .string()
    .min(1)
    .refine((path) => !path.startsWith('/') && !path.split('/').includes('..'), {
      message: 'File path must be relative and cannot contain parent segments',
    }),
  language: z.string().min(1),
  role: z.enum(['entry', 'page', 'component', 'style', 'config', 'asset', 'other']),
  content: z.string(),
  orderIndex: z.number().int().optional(),
});

const createSchema = z.object({
  fileId: z.string().uuid(),
  pageId: z.string().min(1),
  framework: z.enum(FRAMEWORKS as [Framework, ...Framework[]]),
  targetKind: z.enum(['page', 'selection']),
  nodeIds: z.array(z.string()),
  targetHash: z.string().min(1),
  documentRevision: z.number().int().positive(),
  status: z.enum(['done', 'degraded', 'failed']),
  finalCode: z.string().optional(),
  entryFile: z.string().optional(),
  degraded: z.boolean(),
  assets: z.array(assetSchema).default([]),
  files: z.array(fileSchema).default([]),
  model: z.string().optional(),
  provider: z.string().optional(),
  error: z.string().optional(),
  chunks: z.array(chunkSchema).default([]),
  metadata: z.record(z.string(), z.unknown()).optional(),
});

export default defineEventHandler(async (event) => {
  const parsed = createSchema.safeParse(await readBody(event));
  if (!parsed.success) {
    throw toApiError(
      400,
      'validation_error',
      'Invalid code generation payload',
      parsed.error.flatten(),
    );
  }

  const { supabase, user } = await getCloudSupabase(event);
  const file = await supabase
    .from('design_files')
    .select('id')
    .eq('id', parsed.data.fileId)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .single();
  if (file.error || !file.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }

  const saved = await createCodeGenerationRecord({
    supabase,
    userId: user.id,
    fileId: parsed.data.fileId,
    pageId: parsed.data.pageId,
    framework: parsed.data.framework,
    targetKind: parsed.data.targetKind,
    nodeIds: parsed.data.nodeIds,
    targetHash: parsed.data.targetHash,
    documentRevision: parsed.data.documentRevision,
    status: parsed.data.status,
    finalCode: parsed.data.finalCode,
    entryFile: parsed.data.entryFile,
    degraded: parsed.data.degraded,
    assets: parsed.data.assets,
    files: parsed.data.files,
    model: parsed.data.model,
    provider: parsed.data.provider,
    error: parsed.data.error,
    chunks: parsed.data.chunks,
    metadata: parsed.data.metadata,
  });
  setResponseStatus(event, 201);
  return { data: saved };
});
