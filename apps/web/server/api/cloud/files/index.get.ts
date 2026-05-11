import { defineEventHandler, createError } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { mapFileSummary } from '../../../utils/cloud-file-mappers';

export default defineEventHandler(async (event) => {
  const { supabase, user } = await getCloudSupabase(event);
  const { data, error } = await supabase
    .from('design_files')
    .select('id,name,thumbnail_path,revision,created_at,updated_at')
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .order('updated_at', { ascending: false });

  if (error) {
    throw createError({ statusCode: 500, statusMessage: error.message });
  }

  return { data: (data ?? []).map(mapFileSummary) };
});
