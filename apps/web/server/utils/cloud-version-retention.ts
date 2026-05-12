import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';

export const AUTOSAVE_VERSION_RETENTION = 20;

export async function pruneAutosaveVersions(input: {
  supabase: SupabaseClient;
  fileId: string;
  ownerId: string;
  keep?: number;
}) {
  const keep = input.keep ?? AUTOSAVE_VERSION_RETENTION;
  const oldVersions = await input.supabase
    .from('design_file_versions')
    .select('id')
    .eq('file_id', input.fileId)
    .eq('owner_id', input.ownerId)
    .eq('source', 'autosave')
    .order('created_at', { ascending: false })
    .range(keep, 1000);

  if (oldVersions.error) {
    throw createError({ statusCode: 500, statusMessage: oldVersions.error.message });
  }

  const ids = (oldVersions.data ?? [])
    .map((row: { id?: string | null }) => row.id)
    .filter((id): id is string => typeof id === 'string' && id.length > 0);
  if (ids.length === 0) return;

  const deleted = await input.supabase
    .from('design_file_versions')
    .delete()
    .in('id', ids)
    .eq('file_id', input.fileId)
    .eq('owner_id', input.ownerId)
    .eq('source', 'autosave');

  if (deleted.error) {
    throw createError({ statusCode: 500, statusMessage: deleted.error.message });
  }
}
