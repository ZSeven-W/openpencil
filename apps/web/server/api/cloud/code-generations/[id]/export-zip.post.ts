import { defineEventHandler, getRouterParam, createError, setResponseHeaders } from 'h3';
import { getCloudSupabase } from '../../../../utils/cloud-supabase';
import { buildCodeGenerationExportZip } from '../../../../utils/cloud-code-generation-exports';

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  if (!id) throw createError({ statusCode: 400, statusMessage: 'Missing generation ID' });

  const { supabase, user } = await getCloudSupabase(event);
  const result = await buildCodeGenerationExportZip({
    supabase,
    userId: user.id,
    generationId: id,
  });

  setResponseHeaders(event, {
    'Content-Type': 'application/zip',
    'Content-Disposition': `attachment; filename="${result.fileName}"`,
    'Cache-Control': 'no-store',
  });

  return result.bytes;
});
