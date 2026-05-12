import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';
import { randomUUID } from 'node:crypto';
import {
  prepareCloudDocumentForStorage,
  resolveCloudDocumentFromStorage,
} from './cloud-document-storage';
import { recordCloudActivity } from './cloud-activity-events';

export const CLOUD_FILE_SELECT =
  'id,project_id,folder_id,name,document,thumbnail_path,revision,metadata,starred,last_opened_at,deleted_at,created_at,updated_at';

export const CLOUD_FILE_SUMMARY_SELECT =
  'id,project_id,folder_id,name,thumbnail_path,revision,metadata,starred,last_opened_at,deleted_at,created_at,updated_at';

const PROJECT_SELECT = 'id,name,description,icon,color,created_at,updated_at';
const FOLDER_SELECT = 'id,project_id,parent_id,name,sort_order,created_at,updated_at';
const SHARE_SELECT =
  'id,file_id,owner_id,shared_with_user_id,shared_with_email,role,created_at,updated_at';

async function attachUnprojectedFiles(input: {
  supabase: SupabaseClient;
  userId: string;
  projectId: string;
}) {
  const updated = await input.supabase
    .from('design_files')
    .update({ project_id: input.projectId })
    .eq('owner_id', input.userId)
    .is('project_id', null);
  if (updated.error) {
    throw createError({ statusCode: 500, statusMessage: updated.error.message });
  }
}

export async function getDefaultProject(input: { supabase: SupabaseClient; userId: string }) {
  const existing = await input.supabase
    .from('projects')
    .select(PROJECT_SELECT)
    .eq('owner_id', input.userId)
    .is('deleted_at', null)
    .order('created_at', { ascending: true })
    .limit(1)
    .maybeSingle();
  if (existing.error) {
    throw createError({ statusCode: 500, statusMessage: existing.error.message });
  }
  if (existing.data) {
    await attachUnprojectedFiles({
      supabase: input.supabase,
      userId: input.userId,
      projectId: existing.data.id,
    });
    return existing.data;
  }

  const created = await input.supabase
    .from('projects')
    .insert({ owner_id: input.userId, name: 'My Project' })
    .select(PROJECT_SELECT)
    .single();
  if (created.error || !created.data) {
    throw createError({
      statusCode: 500,
      statusMessage: created.error?.message ?? 'Failed to create default project',
    });
  }
  await attachUnprojectedFiles({
    supabase: input.supabase,
    userId: input.userId,
    projectId: created.data.id,
  });
  return created.data;
}

export async function assertProjectAccess(input: {
  supabase: SupabaseClient;
  userId: string;
  projectId: string;
}) {
  const project = await input.supabase
    .from('projects')
    .select('id')
    .eq('id', input.projectId)
    .eq('owner_id', input.userId)
    .is('deleted_at', null)
    .single();
  if (project.error || !project.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud project not found' });
  }
}

export async function assertFolderAccess(input: {
  supabase: SupabaseClient;
  userId: string;
  projectId: string;
  folderId: string;
}) {
  const folder = await input.supabase
    .from('folders')
    .select('id')
    .eq('id', input.folderId)
    .eq('owner_id', input.userId)
    .eq('project_id', input.projectId)
    .is('deleted_at', null)
    .single();
  if (folder.error || !folder.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud folder not found' });
  }
}

export async function assertOwnedCloudFile(input: {
  supabase: SupabaseClient;
  userId: string;
  fileId: string;
}) {
  const file = await input.supabase
    .from('design_files')
    .select('id')
    .eq('id', input.fileId)
    .eq('owner_id', input.userId)
    .is('deleted_at', null)
    .single();
  if (file.error || !file.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }
}

export async function assertCloudFileReadable(input: {
  supabase: SupabaseClient;
  userId: string;
  userEmail?: string | null;
  fileId: string;
}) {
  const owned = await input.supabase
    .from('design_files')
    .select('id,owner_id')
    .eq('id', input.fileId)
    .eq('owner_id', input.userId)
    .is('deleted_at', null)
    .limit(1)
    .maybeSingle();
  if (owned.error) {
    throw createError({ statusCode: 500, statusMessage: owned.error.message });
  }
  if (owned.data) return { ownerId: input.userId, shareRole: null as null | 'viewer' | 'editor' };

  const shareRole = await getCloudFileShareRole(input);
  if (!shareRole) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }

  const file = await input.supabase
    .from('design_files')
    .select('owner_id')
    .eq('id', input.fileId)
    .is('deleted_at', null)
    .single();
  if (file.error || !file.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }
  return { ownerId: file.data.owner_id as string, shareRole };
}

export async function listOwnedFileShares(input: {
  supabase: SupabaseClient;
  userId: string;
  fileId: string;
}) {
  await assertOwnedCloudFile(input);
  const shares = await input.supabase
    .from('design_file_shares')
    .select(SHARE_SELECT)
    .eq('file_id', input.fileId)
    .eq('owner_id', input.userId)
    .is('revoked_at', null)
    .order('created_at', { ascending: false });
  if (shares.error) {
    throw createError({ statusCode: 500, statusMessage: shares.error.message });
  }
  return shares.data ?? [];
}

export async function listSharedCloudFileSummaries(input: {
  supabase: SupabaseClient;
  userId: string;
  userEmail?: string | null;
  search?: string;
  sort: 'updated_desc' | 'updated_asc' | 'name_asc' | 'name_desc' | 'created_desc';
  limit: number;
}) {
  const shareRows = [];
  const byUser = await input.supabase
    .from('design_file_shares')
    .select(`${SHARE_SELECT},revoked_at`)
    .eq('shared_with_user_id', input.userId)
    .is('revoked_at', null);
  if (byUser.error) {
    throw createError({ statusCode: 500, statusMessage: byUser.error.message });
  }
  shareRows.push(...(byUser.data ?? []));

  const email = input.userEmail?.trim().toLowerCase();
  if (email) {
    const byEmail = await input.supabase
      .from('design_file_shares')
      .select(`${SHARE_SELECT},revoked_at`)
      .ilike('shared_with_email', email)
      .is('revoked_at', null);
    if (byEmail.error) {
      throw createError({ statusCode: 500, statusMessage: byEmail.error.message });
    }
    shareRows.push(...(byEmail.data ?? []));
  }

  const roleByFileId = new Map<string, 'viewer' | 'editor'>();
  for (const share of shareRows as Array<{ file_id: string; role: 'viewer' | 'editor' }>) {
    const current = roleByFileId.get(share.file_id);
    if (current !== 'editor') roleByFileId.set(share.file_id, share.role);
  }

  const fileIds = [...roleByFileId.keys()];
  if (fileIds.length === 0) return [];

  let query = input.supabase
    .from('design_files')
    .select(CLOUD_FILE_SUMMARY_SELECT)
    .in('id', fileIds)
    .is('deleted_at', null);
  if (input.search) query = query.ilike('name', `%${input.search}%`);

  if (input.sort === 'updated_asc') {
    query = query.order('updated_at', { ascending: true });
  } else if (input.sort === 'name_asc') {
    query = query.order('name', { ascending: true });
  } else if (input.sort === 'name_desc') {
    query = query.order('name', { ascending: false });
  } else if (input.sort === 'created_desc') {
    query = query.order('created_at', { ascending: false });
  } else {
    query = query.order('updated_at', { ascending: false });
  }

  const files = await query.limit(input.limit);
  if (files.error) {
    throw createError({ statusCode: 500, statusMessage: files.error.message });
  }

  return (files.data ?? []).map((file) => ({
    ...file,
    share_role: roleByFileId.get(file.id),
  }));
}

export async function getCloudFileShareRole(input: {
  supabase: SupabaseClient;
  userId: string;
  userEmail?: string | null;
  fileId: string;
}) {
  const byUser = await input.supabase
    .from('design_file_shares')
    .select('role')
    .eq('file_id', input.fileId)
    .eq('shared_with_user_id', input.userId)
    .is('revoked_at', null)
    .limit(1)
    .maybeSingle();
  if (byUser.error) {
    throw createError({ statusCode: 500, statusMessage: byUser.error.message });
  }
  if (byUser.data?.role) return byUser.data.role as 'viewer' | 'editor';

  const email = input.userEmail?.trim().toLowerCase();
  if (!email) return null;

  const byEmail = await input.supabase
    .from('design_file_shares')
    .select('role')
    .eq('file_id', input.fileId)
    .ilike('shared_with_email', email)
    .is('revoked_at', null)
    .limit(1)
    .maybeSingle();
  if (byEmail.error) {
    throw createError({ statusCode: 500, statusMessage: byEmail.error.message });
  }
  return (byEmail.data?.role as 'viewer' | 'editor' | undefined) ?? null;
}

export async function assertCloudFileEditable(input: {
  supabase: SupabaseClient;
  userId: string;
  userEmail?: string | null;
  fileId: string;
}) {
  const access = await assertCloudFileReadable(input);
  if (access.shareRole === 'viewer') {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }
  return access;
}

export async function resolveProjectAndFolder(input: {
  supabase: SupabaseClient;
  userId: string;
  projectId?: string | null;
  folderId?: string | null;
}) {
  const project = input.projectId
    ? (await assertProjectAccess({
        supabase: input.supabase,
        userId: input.userId,
        projectId: input.projectId,
      }),
      { id: input.projectId })
    : await getDefaultProject({ supabase: input.supabase, userId: input.userId });

  if (input.folderId) {
    await assertFolderAccess({
      supabase: input.supabase,
      userId: input.userId,
      projectId: project.id,
      folderId: input.folderId,
    });
  }

  return {
    projectId: project.id,
    folderId: input.folderId ?? null,
  };
}

export async function copyCloudFile(input: {
  supabase: SupabaseClient;
  userId: string;
  fileId: string;
  name?: string;
  projectId?: string | null;
  folderId?: string | null;
}) {
  const source = await input.supabase
    .from('design_files')
    .select(CLOUD_FILE_SELECT)
    .eq('id', input.fileId)
    .eq('owner_id', input.userId)
    .is('deleted_at', null)
    .single();
  if (source.error || !source.data) {
    throw createError({ statusCode: 404, statusMessage: 'Cloud file not found' });
  }

  const location = await resolveProjectAndFolder({
    supabase: input.supabase,
    userId: input.userId,
    projectId: input.projectId ?? source.data.project_id,
    folderId: input.folderId ?? source.data.folder_id,
  });

  const fileId = randomUUID();
  const document = await resolveCloudDocumentFromStorage(input.supabase, source.data.document);
  const preparedDocument = await prepareCloudDocumentForStorage({
    supabase: input.supabase,
    userId: input.userId,
    fileId,
    revision: 1,
    document,
    attemptId: randomUUID(),
  });

  const copied = await input.supabase
    .from('design_files')
    .insert({
      id: fileId,
      owner_id: input.userId,
      project_id: location.projectId,
      folder_id: location.folderId,
      name: input.name ?? `${source.data.name} Copy`,
      document: preparedDocument.storedDocument,
      thumbnail_path: source.data.thumbnail_path,
      revision: 1,
      metadata: source.data.metadata ?? {},
      starred: false,
    })
    .select(CLOUD_FILE_SELECT)
    .single();

  if (copied.error || !copied.data) {
    throw createError({
      statusCode: 500,
      statusMessage: copied.error?.message ?? 'Failed to copy cloud file',
    });
  }

  const version = await input.supabase.from('design_file_versions').insert({
    file_id: copied.data.id,
    owner_id: input.userId,
    actor_id: input.userId,
    revision: copied.data.revision,
    document: preparedDocument.storedDocument,
    source: 'manual_save',
    label: 'Copied version',
    size_bytes: new TextEncoder().encode(JSON.stringify(preparedDocument.storedDocument)).byteLength,
  });
  if (version.error) {
    throw createError({ statusCode: 500, statusMessage: version.error.message });
  }

  await recordCloudActivity({
    supabase: input.supabase,
    ownerId: input.userId,
    actorId: input.userId,
    fileId: copied.data.id,
    type: 'file_created',
    metadata: {
      sourceFileId: source.data.id,
      sourceRevision: source.data.revision,
      copied: true,
    },
  });

  return {
    row: copied.data,
    document,
  };
}

export { FOLDER_SELECT, PROJECT_SELECT, SHARE_SELECT };
