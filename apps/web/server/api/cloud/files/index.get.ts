import { defineEventHandler, getQuery, createError } from 'h3';
import { z } from 'zod';
import { getCloudSupabase, toApiError } from '../../../utils/cloud-supabase';
import { mapFileSummary } from '../../../utils/cloud-file-mappers';
import {
  CLOUD_FILE_SUMMARY_SELECT,
  getDefaultProject,
  listSharedCloudFileSummaries,
  resolveProjectAndFolder,
} from '../../../utils/cloud-file-management';

const querySchema = z.object({
  projectId: z.string().uuid().optional(),
  folderId: z.preprocess(
    (value) => (value === 'null' ? null : value),
    z.string().uuid().nullable().optional(),
  ),
  view: z.enum(['all', 'recent', 'starred', 'shared', 'trash']).optional().default('all'),
  search: z.string().trim().max(120).optional(),
  sort: z
    .enum(['updated_desc', 'updated_asc', 'name_asc', 'name_desc', 'created_desc'])
    .optional()
    .default('updated_desc'),
  limit: z.coerce.number().int().positive().max(200).optional().default(100),
});

export default defineEventHandler(async (event) => {
  const parsed = querySchema.safeParse(getQuery(event));
  if (!parsed.success) {
    throw toApiError(400, 'validation_error', 'Invalid cloud file query', parsed.error.flatten());
  }

  const { supabase, user } = await getCloudSupabase(event);
  if (parsed.data.view === 'shared') {
    const sharedFiles = await listSharedCloudFileSummaries({
      supabase,
      userId: user.id,
      userEmail: user.email,
      search: parsed.data.search,
      sort: parsed.data.sort,
      limit: parsed.data.limit,
    });
    return { data: sharedFiles.map(mapFileSummary) };
  }

  const defaultProject = parsed.data.projectId
    ? null
    : await getDefaultProject({ supabase, userId: user.id });
  const projectId = parsed.data.projectId ?? defaultProject?.id;

  if (parsed.data.projectId || parsed.data.folderId) {
    await resolveProjectAndFolder({
      supabase,
      userId: user.id,
      projectId,
      folderId: parsed.data.folderId,
    });
  }

  let query = supabase
    .from('design_files')
    .select(CLOUD_FILE_SUMMARY_SELECT)
    .eq('owner_id', user.id);

  if (projectId) query = query.eq('project_id', projectId);
  if (parsed.data.folderId !== undefined) {
    query = parsed.data.folderId === null
      ? query.is('folder_id', null)
      : query.eq('folder_id', parsed.data.folderId);
  }

  if (parsed.data.view === 'trash') {
    query = query.not('deleted_at', 'is', null);
  } else {
    query = query.is('deleted_at', null);
  }
  if (parsed.data.view === 'starred') query = query.eq('starred', true);
  if (parsed.data.view === 'recent') query = query.not('last_opened_at', 'is', null);
  if (parsed.data.search) query = query.ilike('name', `%${parsed.data.search}%`);

  let sortedQuery = query;
  if (parsed.data.view === 'recent') {
    sortedQuery = sortedQuery.order('last_opened_at', { ascending: false });
  } else if (parsed.data.sort === 'updated_asc') {
    sortedQuery = sortedQuery.order('updated_at', { ascending: true });
  } else if (parsed.data.sort === 'name_asc') {
    sortedQuery = sortedQuery.order('name', { ascending: true });
  } else if (parsed.data.sort === 'name_desc') {
    sortedQuery = sortedQuery.order('name', { ascending: false });
  } else if (parsed.data.sort === 'created_desc') {
    sortedQuery = sortedQuery.order('created_at', { ascending: false });
  } else {
    sortedQuery = sortedQuery.order('updated_at', { ascending: false });
  }

  const { data, error } = await sortedQuery.limit(parsed.data.limit);

  if (error) {
    throw createError({ statusCode: 500, statusMessage: error.message });
  }

  return { data: (data ?? []).map(mapFileSummary) };
});
