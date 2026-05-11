import { defineEventHandler, getRouterParam, createError } from 'h3';
import { getCloudSupabase } from '../../../../utils/cloud-supabase';
import { mapFileVersion } from '../../../../utils/cloud-file-mappers';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const { data, error } = await supabase
    .from('design_file_versions')
    .select('id,file_id,revision,source,label,created_at')
    .eq('file_id', id)
    .eq('owner_id', user.id)
    .order('revision', { ascending: false })
    .limit(100);

  if (error) {
    throw createError({ statusCode: 500, statusMessage: error.message });
  }

  return { data: (data ?? []).map(mapFileVersion) };
});

