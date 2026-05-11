import { defineEventHandler, readMultipartFormData, createError, setResponseStatus } from 'h3';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';

export default defineEventHandler(async (event) => {
  const { supabase, user } = await getCloudSupabase(event);
  const parts = await readMultipartFormData(event);
  if (!parts) {
    throw toApiError(400, 'validation_error', 'Expected multipart form data');
  }

  const fileId = parts.find((part) => part.name === 'fileId')?.data.toString();
  const kind = parts.find((part) => part.name === 'kind')?.data.toString() ?? 'codegen_asset';
  const filePart = parts.find((part) => part.name === 'file' && part.filename);

  if (!fileId || !filePart?.filename || !filePart.type) {
    throw toApiError(400, 'validation_error', 'Missing fileId or file payload');
  }

  const file = await supabase
    .from('design_files')
    .select('id')
    .eq('id', fileId)
    .eq('owner_id', user.id)
    .is('deleted_at', null)
    .single();
  if (file.error || !file.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }

  const safeName = filePart.filename.replace(/[^a-zA-Z0-9._-]+/g, '-');
  const path = `${user.id}/${fileId}/${Date.now()}-${safeName}`;
  const uploaded = await supabase.storage
    .from('openpencil-assets')
    .upload(path, filePart.data, {
      contentType: filePart.type,
      upsert: false,
    });
  if (uploaded.error) {
    throw createError({ statusCode: 500, statusMessage: uploaded.error.message });
  }

  const asset = await supabase
    .from('file_assets')
    .insert({
      file_id: fileId,
      owner_id: user.id,
      kind,
      path,
      mime_type: filePart.type,
      size_bytes: filePart.data.byteLength,
      metadata: { originalName: filePart.filename },
    })
    .select('id,path,mime_type,size_bytes,created_at')
    .single();
  if (asset.error || !asset.data) {
    throw createError({
      statusCode: 500,
      statusMessage: asset.error?.message ?? 'Failed to record asset',
    });
  }

  setResponseStatus(event, 201);
  return { data: asset.data };
});

