import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';
import type { PenDocument } from '../../src/types/pen';

export const CLOUD_DOCUMENT_STORAGE_BUCKET = 'openpencil-assets';
export const CLOUD_DOCUMENT_REF_MARKER = '__openpencilCloudDocumentRef';
export const DEFAULT_INLINE_CLOUD_DOCUMENT_MAX_BYTES = 4 * 1024 * 1024;

export interface CloudDocumentRef {
  [CLOUD_DOCUMENT_REF_MARKER]: 1;
  bucket: string;
  path: string;
  sizeBytes: number;
  contentType?: 'application/json';
}

export type StoredCloudDocument = PenDocument | CloudDocumentRef;

export interface PreparedCloudDocument {
  storedDocument: StoredCloudDocument;
  sizeBytes: number;
  isExternal: boolean;
}

function byteLength(value: string): number {
  return new TextEncoder().encode(value).byteLength;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function isCloudDocumentRef(value: unknown): value is CloudDocumentRef {
  if (!isRecord(value)) return false;
  return (
    value[CLOUD_DOCUMENT_REF_MARKER] === 1 &&
    typeof value.bucket === 'string' &&
    value.bucket.length > 0 &&
    typeof value.path === 'string' &&
    value.path.length > 0 &&
    typeof value.sizeBytes === 'number' &&
    Number.isFinite(value.sizeBytes)
  );
}

export async function prepareCloudDocumentForStorage(input: {
  supabase: SupabaseClient;
  userId: string;
  fileId: string;
  revision: number;
  document: PenDocument;
  inlineMaxBytes?: number;
  attemptId?: string;
}): Promise<PreparedCloudDocument> {
  const json = JSON.stringify(input.document);
  const sizeBytes = byteLength(json);
  const inlineMaxBytes = input.inlineMaxBytes ?? DEFAULT_INLINE_CLOUD_DOCUMENT_MAX_BYTES;

  if (sizeBytes <= inlineMaxBytes) {
    return {
      storedDocument: input.document,
      sizeBytes,
      isExternal: false,
    };
  }

  const attemptId = input.attemptId ?? crypto.randomUUID();
  const safeAttemptId = attemptId.replace(/[^a-zA-Z0-9._-]+/g, '-');
  const path = `${input.userId}/${input.fileId}/documents/rev-${input.revision}-${safeAttemptId}.json`;
  const uploaded = await input.supabase.storage
    .from(CLOUD_DOCUMENT_STORAGE_BUCKET)
    .upload(path, json, {
      contentType: 'application/json',
      upsert: false,
    });

  if (uploaded.error) {
    throw createError({
      statusCode: 500,
      statusMessage: uploaded.error.message,
    });
  }

  return {
    storedDocument: {
      [CLOUD_DOCUMENT_REF_MARKER]: 1,
      bucket: CLOUD_DOCUMENT_STORAGE_BUCKET,
      path,
      sizeBytes,
      contentType: 'application/json',
    },
    sizeBytes,
    isExternal: true,
  };
}

export async function resolveCloudDocumentFromStorage(
  supabase: SupabaseClient,
  storedDocument: unknown,
): Promise<PenDocument> {
  if (!isCloudDocumentRef(storedDocument)) {
    return storedDocument as PenDocument;
  }

  const downloaded = await supabase.storage.from(storedDocument.bucket).download(storedDocument.path);
  if (downloaded.error || !downloaded.data) {
    throw createError({
      statusCode: 500,
      statusMessage: downloaded.error?.message ?? 'Failed to load cloud document',
    });
  }

  try {
    return JSON.parse(await downloaded.data.text()) as PenDocument;
  } catch {
    throw createError({
      statusCode: 500,
      statusMessage: 'Stored cloud document is invalid JSON',
    });
  }
}
