import type { SupabaseClient } from '@supabase/supabase-js';
import { toApiError } from './cloud-supabase';

export function isOptimisticUpdateMiss(data: unknown, error: unknown): boolean {
  if (data) return false;
  if (!error) return true;
  const code = (error as { code?: string }).code;
  return code === 'PGRST116';
}

export async function throwCloudRevisionConflict(
  supabase: SupabaseClient,
  fileId: string,
  userId: string,
): Promise<never> {
  const current = await supabase
    .from('design_files')
    .select('revision')
    .eq('id', fileId)
    .eq('owner_id', userId)
    .maybeSingle();

  throw toApiError(409, 'revision_conflict', 'Cloud file has a newer revision', {
    serverRevision: current.data ? Number(current.data.revision) : null,
  });
}
