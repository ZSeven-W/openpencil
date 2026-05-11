import type { PenDocument } from '@/types/pen';
import type { CloudFileRecord, CloudFileSummary, CloudFileVersion } from '@/types/cloud';
import { cloudFetch } from './cloud-fetch';
import { stringifyCloudFilePayload } from './cloud-file-payload';

export async function listCloudFiles(): Promise<CloudFileSummary[]> {
  const res = await cloudFetch<{ data: CloudFileSummary[] }>('/api/cloud/files');
  return res.data;
}

export async function createCloudFile(input: {
  name: string;
  document: PenDocument;
  source?: 'import' | 'manual_save';
}): Promise<CloudFileRecord> {
  const res = await cloudFetch<{ data: CloudFileRecord }>('/api/cloud/files', {
    method: 'POST',
    body: stringifyCloudFilePayload(input),
  });
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

export async function listCloudFileVersions(id: string): Promise<CloudFileVersion[]> {
  const res = await cloudFetch<{ data: CloudFileVersion[] }>(
    `/api/cloud/files/${encodeURIComponent(id)}/versions`,
  );
  return res.data;
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
