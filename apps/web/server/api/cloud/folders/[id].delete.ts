import { defineEventHandler, getRouterParam, createError, setResponseStatus } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing folder ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const existing = await supabase
    .from('folders')
    .select('id')
    .eq('id', id)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .single();
  if (existing.error || !existing.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud folder not found' });
  }

  const deletedAt = new Date().toISOString();
  const deleted = await supabase
    .from('folders')
    .update({ deleted_at: deletedAt })
    .eq('id', id)
    .eq('owner_id', user.id);
  if (deleted.error) {
    throw createError({ statusCode: 500, statusMessage: deleted.error.message });
  }

  const childFolders = await supabase
    .from('folders')
    .update({ parent_id: null })
    .eq('parent_id', id)
    .eq('owner_id', user.id)
    .is('deleted_at', null);
  if (childFolders.error) {
    throw createError({ statusCode: 500, statusMessage: childFolders.error.message });
  }

  const files = await supabase
    .from('design_files')
    .update({ folder_id: null })
    .eq('folder_id', id)
    .eq('owner_id', user.id);
  if (files.error) {
    throw createError({ statusCode: 500, statusMessage: files.error.message });
  }

  setResponseStatus(event, 204);
  return null;
});
