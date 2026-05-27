import type { SupabaseClient } from '@supabase/supabase-js';
import { applyDocumentPatches, type DocumentPatch } from '@zseven-w/pen-core';
import { createError } from 'h3';
import type { PenDocument } from '../../src/types/pen';
import { resolveCloudDocumentFromStorage } from './cloud-document-storage';

interface ChangeRow {
  revision: number;
  patches: DocumentPatch[];
}

export interface CloudDocumentChangeSource {
  id: string;
  owner_id?: string | null;
  document: unknown;
  revision: number;
  checkpoint_revision?: number | null;
}

export async function resolveCloudDocumentWithChanges(
  supabase: SupabaseClient,
  row: CloudDocumentChangeSource,
): Promise<PenDocument> {
  let document = await resolveCloudDocumentFromStorage(supabase, row.document);
  const currentRevision = Number(row.revision);
  const checkpointRevision = Number(row.checkpoint_revision ?? currentRevision);
  if (!row.owner_id || checkpointRevision >= currentRevision) {
    return document;
  }

  const changes = await supabase
    .from('design_file_changes')
    .select('revision,patches')
    .eq('file_id', row.id)
    .eq('owner_id', row.owner_id)
    .gt('revision', checkpointRevision)
    .lte('revision', currentRevision)
    .order('revision', { ascending: true });

  if (changes.error) {
    throw createError({ statusCode: 500, statusMessage: changes.error.message });
  }

  for (const change of (changes.data ?? []) as ChangeRow[]) {
    document = applyDocumentPatches(document, change.patches);
  }
  return document;
}

export function jsonSizeBytes(value: unknown): number {
  return new TextEncoder().encode(JSON.stringify(value)).byteLength;
}
