import type { PenDocument } from '@/types/pen';
import type {
  CloudFileRecord,
  CloudFileSort,
  CloudFileSummary,
  CloudFileVersion,
  CloudFileView,
  CloudFileShare,
  CloudShareRole,
  CloudFolder,
  CloudProject,
  CloudActivityEvent,
  CloudActivityEventType,
  CloudActivityPage,
} from '@/types/cloud';
import { cloudFetch } from './cloud-fetch';
import { stringifyCloudFilePayload } from './cloud-file-payload';

function buildQuery(input: Record<string, string | number | boolean | null | undefined>): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(input)) {
    if (value !== undefined) params.set(key, value === null ? 'null' : String(value));
  }
  const query = params.toString();
  return query ? `?${query}` : '';
}

export async function listCloudProjects(): Promise<CloudProject[]> {
  const res = await cloudFetch<{ data: CloudProject[] }>('/api/cloud/projects');
  return res.data;
}

export async function createCloudProject(input: {
  name: string;
  description?: string | null;
  icon?: string | null;
  color?: string | null;
}): Promise<CloudProject> {
  const res = await cloudFetch<{ data: CloudProject }>('/api/cloud/projects', {
    method: 'POST',
    body: JSON.stringify(input),
  });
  return res.data;
}

export async function updateCloudProject(input: {
  id: string;
  name?: string;
  description?: string | null;
  icon?: string | null;
  color?: string | null;
}): Promise<CloudProject> {
  const res = await cloudFetch<{ data: CloudProject }>(
    `/api/cloud/projects/${encodeURIComponent(input.id)}`,
    {
      method: 'PATCH',
      body: JSON.stringify(input),
    },
  );
  return res.data;
}

export async function deleteCloudProject(id: string): Promise<void> {
  await cloudFetch(`/api/cloud/projects/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

export async function listCloudFolders(input: {
  projectId?: string;
  parentId?: string | null;
} = {}): Promise<CloudFolder[]> {
  const res = await cloudFetch<{ data: CloudFolder[] }>(
    `/api/cloud/folders${buildQuery(input)}`,
  );
  return res.data;
}

export async function createCloudFolder(input: {
  projectId?: string | null;
  parentId?: string | null;
  name: string;
  sortOrder?: number;
}): Promise<CloudFolder> {
  const res = await cloudFetch<{ data: CloudFolder }>('/api/cloud/folders', {
    method: 'POST',
    body: JSON.stringify(input),
  });
  return res.data;
}

export async function updateCloudFolder(input: {
  id: string;
  projectId?: string | null;
  parentId?: string | null;
  name?: string;
  sortOrder?: number;
}): Promise<CloudFolder> {
  const res = await cloudFetch<{ data: CloudFolder }>(
    `/api/cloud/folders/${encodeURIComponent(input.id)}`,
    {
      method: 'PATCH',
      body: JSON.stringify(input),
    },
  );
  return res.data;
}

export async function deleteCloudFolder(id: string): Promise<void> {
  await cloudFetch(`/api/cloud/folders/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

export async function listCloudFiles(input: {
  projectId?: string;
  folderId?: string | null;
  view?: CloudFileView;
  search?: string;
  sort?: CloudFileSort;
  limit?: number;
} = {}): Promise<CloudFileSummary[]> {
  const res = await cloudFetch<{ data: CloudFileSummary[] }>(
    `/api/cloud/files${buildQuery(input)}`,
  );
  return res.data;
}

export async function createCloudFile(input: {
  name: string;
  document: PenDocument;
  projectId?: string | null;
  folderId?: string | null;
  metadata?: Record<string, unknown>;
  source?: 'import' | 'manual_save';
}): Promise<CloudFileRecord> {
  const res = await cloudFetch<{ data: CloudFileRecord }>('/api/cloud/files', {
    method: 'POST',
    body: stringifyCloudFilePayload(input),
  });
  return res.data;
}

export async function updateCloudFileMetadata(input: {
  id: string;
  name?: string;
  projectId?: string | null;
  folderId?: string | null;
  metadata?: Record<string, unknown>;
  starred?: boolean;
}): Promise<CloudFileRecord> {
  const res = await cloudFetch<{ data: CloudFileRecord }>(
    `/api/cloud/files/${encodeURIComponent(input.id)}`,
    {
      method: 'PATCH',
      body: JSON.stringify(input),
    },
  );
  return res.data;
}

export async function getCloudFile(id: string): Promise<CloudFileRecord> {
  const res = await cloudFetch<{ data: CloudFileRecord }>(
    `/api/cloud/files/${encodeURIComponent(id)}`,
  );
  return res.data;
}

export async function saveCloudFile(input: {
  id: string;
  name?: string;
  document: PenDocument;
  expectedRevision: number;
  source?: 'manual_save' | 'autosave' | 'code_generation' | 'restore';
  label?: string;
  snapshot?: boolean;
  force?: boolean;
}): Promise<CloudFileRecord> {
  const res = await cloudFetch<{ data: CloudFileRecord }>(
    `/api/cloud/files/${encodeURIComponent(input.id)}`,
    {
      method: 'PATCH',
      body: stringifyCloudFilePayload(input),
    },
  );
  return res.data;
}

export async function deleteCloudFile(id: string): Promise<void> {
  await cloudFetch<{ ok: true }>(`/api/cloud/files/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

export async function copyCloudFile(input: {
  id: string;
  name?: string;
  projectId?: string | null;
  folderId?: string | null;
}): Promise<CloudFileRecord> {
  const res = await cloudFetch<{ data: CloudFileRecord }>(
    `/api/cloud/files/${encodeURIComponent(input.id)}/copy`,
    {
      method: 'POST',
      body: JSON.stringify(input),
    },
  );
  return res.data;
}

export async function restoreCloudFile(id: string): Promise<CloudFileRecord> {
  const res = await cloudFetch<{ data: CloudFileRecord }>(
    `/api/cloud/files/${encodeURIComponent(id)}/restore`,
    { method: 'POST' },
  );
  return res.data;
}

export async function permanentlyDeleteCloudFile(id: string): Promise<void> {
  await cloudFetch(`/api/cloud/files/${encodeURIComponent(id)}/permanent-delete`, {
    method: 'DELETE',
  });
}

export async function listCloudFileVersions(id: string): Promise<CloudFileVersion[]> {
  const res = await cloudFetch<{ data: CloudFileVersion[] }>(
    `/api/cloud/files/${encodeURIComponent(id)}/versions`,
  );
  return res.data;
}

export async function updateCloudFileVersionLabel(input: {
  fileId: string;
  versionId: string;
  label: string | null;
}): Promise<CloudFileVersion> {
  const res = await cloudFetch<{ data: CloudFileVersion }>(
    `/api/cloud/files/${encodeURIComponent(input.fileId)}/versions/${encodeURIComponent(input.versionId)}`,
    {
      method: 'PATCH',
      body: JSON.stringify({ label: input.label }),
    },
  );
  return res.data;
}

export async function listCloudFileActivity(
  id: string,
  input: {
    type?: CloudActivityEventType | 'all';
    cursor?: string | null;
    limit?: number;
  } = {},
): Promise<CloudActivityPage> {
  const query = buildQuery({
    type: input.type && input.type !== 'all' ? input.type : undefined,
    cursor: input.cursor ?? undefined,
    limit: input.limit,
  });
  const res = await cloudFetch<{
    data: CloudActivityEvent[];
    meta?: { nextCursor?: string | null; limit?: number };
  }>(
    `/api/cloud/files/${encodeURIComponent(id)}/activity${query}`,
  );
  return {
    data: res.data,
    nextCursor: res.meta?.nextCursor ?? null,
    limit: res.meta?.limit ?? input.limit ?? res.data.length,
  };
}

export async function listCloudFileShares(id: string): Promise<CloudFileShare[]> {
  const res = await cloudFetch<{ data: CloudFileShare[] }>(
    `/api/cloud/files/${encodeURIComponent(id)}/shares`,
  );
  return res.data;
}

export async function createCloudFileShare(input: {
  fileId: string;
  email?: string;
  userId?: string;
  role?: CloudShareRole;
}): Promise<CloudFileShare> {
  const res = await cloudFetch<{ data: CloudFileShare }>(
    `/api/cloud/files/${encodeURIComponent(input.fileId)}/shares`,
    {
      method: 'POST',
      body: JSON.stringify(input),
    },
  );
  return res.data;
}

export async function revokeCloudFileShare(input: {
  fileId: string;
  shareId: string;
}): Promise<void> {
  await cloudFetch(
    `/api/cloud/files/${encodeURIComponent(input.fileId)}/shares/${encodeURIComponent(input.shareId)}`,
    { method: 'DELETE' },
  );
}

export async function restoreCloudFileVersion(input: {
  fileId: string;
  versionId: string;
}): Promise<CloudFileRecord> {
  const res = await cloudFetch<{ data: CloudFileRecord }>(
    `/api/cloud/files/${encodeURIComponent(input.fileId)}/restore-version`,
    {
      method: 'POST',
      body: JSON.stringify({ versionId: input.versionId }),
    },
  );
  return res.data;
}
