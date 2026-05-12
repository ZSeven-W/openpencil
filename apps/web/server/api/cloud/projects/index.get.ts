import { defineEventHandler, createError } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { mapProject } from '../../../utils/cloud-file-mappers';
import { getDefaultProject, PROJECT_SELECT } from '../../../utils/cloud-file-management';

export default defineEventHandler(async (event) => {
  const { supabase, user } = await getCloudSupabase(event);
  await getDefaultProject({ supabase, userId: user.id });

  const { data, error } = await supabase
    .from('projects')
    .select(PROJECT_SELECT)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .order('updated_at', { ascending: false });

  if (error) throw createError({ statusCode: 500, statusMessage: error.message });
  return { data: (data ?? []).map(mapProject) };
});
