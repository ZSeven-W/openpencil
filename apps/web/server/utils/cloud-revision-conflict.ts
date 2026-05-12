import type { SupabaseClient } from '@supabase/supabase-js';
import { toApiError } from './cloud-supabase';

export interface CloudRevisionConflictDetails {
  fileId: string;
  expectedRevision: number;
  serverRevision: number | null;
}

export function isOptimisticUpdateMiss(data: unknown, error: unknown): boolean {
  if (data) return false;
  if (!error) return true;
  const code = (error as { code?: string }).code;
  return code === 'PGRST116';
}

export function createCloudRevisionConflictDetails(
  fileId: string,
  expectedRevision: number,
  serverRevision: number | null,
): CloudRevisionConflictDetails {
  return { fileId, expectedRevision, serverRevision };
}

export async function throwCloudRevisionConflict(
  supabase: SupabaseClient,
  fileId: string,
  userId: string,
  expectedRevision: number,
): Promise<never> {
  const current = await supabase
    .from('design_files')
    .select('revision')
    .eq('id', fileId)
    .eq('owner_id', userId)
    .maybeSingle();

  throw toApiError(409, 'revision_conflict', 'Cloud file has a newer revision', {
    ...createCloudRevisionConflictDetails(
      fileId,
      expectedRevision,
      current.data ? Number(current.data.revision) : null,
    ),
  });
}
