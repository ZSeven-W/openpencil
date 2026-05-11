import { defineEventHandler, getRouterParam, createError } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { mapFileRecord } from '../../../utils/cloud-file-mappers';
import { resolveCloudDocumentFromStorage } from '../../../utils/cloud-document-storage';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const { data, error } = await supabase
    .from('design_files')
    .select('id,name,document,thumbnail_path,revision,created_at,updated_at')
    .eq('id', id)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .single();

  if (error || !data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }

  const document = await resolveCloudDocumentFromStorage(supabase, data.document);
  return { data: mapFileRecord({ ...data, document }) };
});
