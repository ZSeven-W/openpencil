import { defineEventHandler, getRouterParam, createError } from 'h3';
import { getCloudSupabase } from '../../../utils/cloud-supabase';
import { mapFileRecord } from '../../../utils/cloud-file-mappers';
import { resolveCloudDocumentFromStorage } from '../../../utils/cloud-document-storage';
import { CLOUD_FILE_SELECT, getCloudFileShareRole } from '../../../utils/cloud-file-management';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing file ID' });

  const { supabase, user } = await getCloudSupabase(event);
  let { data, error } = await supabase
    .from('design_files')
    .select(CLOUD_FILE_SELECT)
    .eq('id', id)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .single();

  let shareRole: 'viewer' | 'editor' | null = null;
  if (error || !data) {
    shareRole = await getCloudFileShareRole({
      supabase,
      userId: user.id,
      userEmail: user.email,
      fileId: id,
    });
    if (!shareRole) {
      throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
    }
    const shared = await supabase
      .from('design_files')
      .select(CLOUD_FILE_SELECT)
      .eq('id', id)
      .is('deleted_at', null)
      .single();
    data = shared.data;
    error = shared.error;
    if (error || !data) {
      throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
    }
  }

  const document = await resolveCloudDocumentFromStorage(supabase, data.document);
  let openedData = data;
  if (!shareRole || shareRole === 'editor') {
    const opened = await supabase
      .from('design_files')
      .update({ last_opened_at: new Date().toISOString() })
      .eq('id', id)
      .select(CLOUD_FILE_SELECT)
      .single();
    if (opened.error || !opened.data) {
      throw createError({
        statusCode: 500,
        statusMessage: opened.error?.message ?? 'Failed to update recent file timestamp',
      });
    }
    openedData = opened.data;
  }

  return { data: mapFileRecord({ ...openedData, document, share_role: shareRole }) };
});
