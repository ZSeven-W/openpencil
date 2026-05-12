import { defineEventHandler, getRouterParam, createError, setResponseStatus } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { getDefaultProject, PROJECT_SELECT } from '../../../utils/cloud-file-management';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing project ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const existing = await supabase
    .from('projects')
    .select(PROJECT_SELECT)
    .eq('id', id)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .single();
  if (existing.error || !existing.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud project not found' });
  }

  const deletedAt = new Date().toISOString();
  const deleted = await supabase
    .from('projects')
    .update({ deleted_at: deletedAt })
    .eq('id', id)
    .eq('owner_id', user.id);
  if (deleted.error) {
    throw createError({ statusCode: 500, statusMessage: deleted.error.message });
  }

  const folders = await supabase
    .from('folders')
    .update({ deleted_at: deletedAt })
    .eq('project_id', id)
    .eq('owner_id', user.id)
    .is('deleted_at', null);
  if (folders.error) {
    throw createError({ statusCode: 500, statusMessage: folders.error.message });
  }

  const fallback = await getDefaultProject({ supabase, userId: user.id });
  const files = await supabase
    .from('design_files')
    .update({ project_id: fallback.id, folder_id: null })
    .eq('project_id', id)
    .eq('owner_id', user.id);
  if (files.error) {
    throw createError({ statusCode: 500, statusMessage: files.error.message });
  }

  setResponseStatus(event, 204);
  return null;
});
