import { describe, expect, it, vi } from 'vitest';
import type { PenDocument } from '../../../src/types/pen';
import { CLOUD_DOCUMENT_REF_MARKER } from '../cloud-document-storage';
import {
  assertFolderAccess,
  copyCloudFile,
  getDefaultProject,
  resolveProjectAndFolder,
} from '../cloud-file-management';

function makeDocument(name: string): PenDocument {
  return {
    version: '1.0.0',
    children: [{ id: 'node-1', type: 'text', name, content: name }],
  } as PenDocument;
}

function makeSingleQuery(row: unknown, error: unknown = null) {
  const query = {
    select: vi.fn(() => query),
    eq: vi.fn(() => query),
    is: vi.fn(() => query),
    order: vi.fn(() => query),
    limit: vi.fn(() => query),
    single: vi.fn(async () => ({ data: row, error })),
    maybeSingle: vi.fn(async () => ({ data: row, error })),
  };
  return query;
}

describe('cloud file management helpers', () => {
  it('returns an existing default project without creating another one', async () => {
    const existing = {
      id: 'project-1',
      name: 'Workspace',
      description: null,
      icon: null,
      color: null,
      created_at: '2026-05-12T08:00:00.000Z',
      updated_at: '2026-05-12T08:00:00.000Z',
    };
    const query = makeSingleQuery(existing);
    const attachQuery = {
      update: vi.fn(() => attachQuery),
      eq: vi.fn(() => attachQuery),
      is: vi.fn(async () => ({ data: null, error: null })),
    };
    const insert = vi.fn();
    const from = vi
      .fn()
      .mockReturnValueOnce({ select: query.select, insert })
      .mockReturnValueOnce({ update: attachQuery.update });

    await expect(
      getDefaultProject({ supabase: { from } as never, userId: 'user-1' }),
    ).resolves.toEqual(existing);

    expect(from).toHaveBeenCalledWith('projects');
    expect(query.eq).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(attachQuery.update).toHaveBeenCalledWith({ project_id: 'project-1' });
    expect(attachQuery.eq).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(attachQuery.is).toHaveBeenCalledWith('project_id', null);
    expect(insert).not.toHaveBeenCalled();
  });

  it('creates a default project when the user has none', async () => {
    const lookup = makeSingleQuery(null);
    const created = {
      id: 'project-1',
      name: 'My Project',
      description: null,
      icon: null,
      color: null,
      created_at: '2026-05-12T08:00:00.000Z',
      updated_at: '2026-05-12T08:00:00.000Z',
    };
    const createSingle = vi.fn(async () => ({ data: created, error: null }));
    const createSelect = vi.fn(() => ({ single: createSingle }));
    const insert = vi.fn(() => ({ select: createSelect }));
    const attachQuery = {
      update: vi.fn(() => attachQuery),
      eq: vi.fn(() => attachQuery),
      is: vi.fn(async () => ({ data: null, error: null })),
    };
    const from = vi
      .fn()
      .mockReturnValueOnce({ select: lookup.select, insert })
      .mockReturnValueOnce({ insert })
      .mockReturnValueOnce({ update: attachQuery.update });

    await expect(
      getDefaultProject({ supabase: { from } as never, userId: 'user-1' }),
    ).resolves.toEqual(created);

    expect(insert).toHaveBeenCalledWith({ owner_id: 'user-1', name: 'My Project' });
    expect(createSelect).toHaveBeenCalled();
    expect(attachQuery.update).toHaveBeenCalledWith({ project_id: 'project-1' });
  });

  it('rejects folders that do not belong to the target project', async () => {
    const folderQuery = makeSingleQuery(null);
    const from = vi.fn(() => ({ select: folderQuery.select }));

    await expect(
      assertFolderAccess({
        supabase: { from } as never,
        userId: 'user-1',
        projectId: 'project-1',
        folderId: 'folder-2',
      }),
    ).rejects.toMatchObject({
      statusCode: 404,
      statusMessage: 'Cloud folder not found',
    });

    expect(folderQuery.eq).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(folderQuery.eq).toHaveBeenCalledWith('project_id', 'project-1');
  });

  it('resolves explicit project and folder after access checks', async () => {
    const projectQuery = makeSingleQuery({ id: 'project-1' });
    const folderQuery = makeSingleQuery({ id: 'folder-1' });
    const from = vi.fn((table: string) => {
      if (table === 'projects') return { select: projectQuery.select };
      if (table === 'folders') return { select: folderQuery.select };
      throw new Error(`Unexpected table ${table}`);
    });

    await expect(
      resolveProjectAndFolder({
        supabase: { from } as never,
        userId: 'user-1',
        projectId: 'project-1',
        folderId: 'folder-1',
      }),
    ).resolves.toEqual({ projectId: 'project-1', folderId: 'folder-1' });

    expect(projectQuery.eq).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(folderQuery.eq).toHaveBeenCalledWith('project_id', 'project-1');
  });

  it('copies a cloud file into a new row and creates the initial copied version', async () => {
    const document = makeDocument('inline');
    const sourceRow = {
      id: 'source-1',
      project_id: 'project-1',
      folder_id: 'folder-1',
      name: 'Dashboard',
      document,
      thumbnail_path: null,
      revision: 4,
      metadata: { source: 'test' },
      starred: true,
      last_opened_at: null,
      deleted_at: null,
      created_at: '2026-05-12T08:00:00.000Z',
      updated_at: '2026-05-12T08:00:00.000Z',
    };
    const copiedRow = {
      ...sourceRow,
      id: 'copy-1',
      name: 'Dashboard Copy',
      revision: 1,
      starred: false,
    };
    const sourceQuery = makeSingleQuery(sourceRow);
    const projectQuery = makeSingleQuery({ id: 'project-1' });
    const folderQuery = makeSingleQuery({ id: 'folder-1' });
    const copiedSingle = vi.fn(async () => ({ data: copiedRow, error: null }));
    const copiedSelect = vi.fn(() => ({ single: copiedSingle }));
    const fileInsert = vi.fn(() => ({ select: copiedSelect }));
    const versionInsert = vi.fn(async () => ({ data: null, error: null }));
    const from = vi.fn((table: string) => {
      if (table === 'design_files') {
        return { select: sourceQuery.select, insert: fileInsert };
      }
      if (table === 'projects') return { select: projectQuery.select };
      if (table === 'folders') return { select: folderQuery.select };
      if (table === 'design_file_versions') return { insert: versionInsert };
      throw new Error(`Unexpected table ${table}`);
    });

    const copied = await copyCloudFile({
      supabase: {
        from,
        storage: { from: vi.fn() },
      } as never,
      userId: 'user-1',
      fileId: 'source-1',
    });

    expect(copied.document).toEqual(document);
    expect(fileInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        owner_id: 'user-1',
        project_id: 'project-1',
        folder_id: 'folder-1',
        name: 'Dashboard Copy',
        document,
        revision: 1,
        metadata: { source: 'test' },
        starred: false,
      }),
    );
    expect(versionInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        file_id: 'copy-1',
        owner_id: 'user-1',
        revision: 1,
        document,
        label: 'Copied version',
      }),
    );
  });

  it('resolves external document references before copying a cloud file', async () => {
    const document = makeDocument('external');
    const sourceRow = {
      id: 'source-1',
      project_id: 'project-1',
      folder_id: null,
      name: 'Large File',
      document: {
        [CLOUD_DOCUMENT_REF_MARKER]: 1,
        bucket: 'openpencil-assets',
        path: 'user-1/source-1/documents/rev-4.json',
        sizeBytes: 1024,
      },
      thumbnail_path: null,
      revision: 4,
      metadata: {},
      starred: false,
      last_opened_at: null,
      deleted_at: null,
      created_at: '2026-05-12T08:00:00.000Z',
      updated_at: '2026-05-12T08:00:00.000Z',
    };
    const copiedRow = {
      ...sourceRow,
      id: 'copy-1',
      document,
      revision: 1,
    };
    const sourceQuery = makeSingleQuery(sourceRow);
    const projectQuery = makeSingleQuery({ id: 'project-1' });
    const copiedSingle = vi.fn(async () => ({ data: copiedRow, error: null }));
    const copiedSelect = vi.fn(() => ({ single: copiedSingle }));
    const fileInsert = vi.fn(() => ({ select: copiedSelect }));
    const versionInsert = vi.fn(async () => ({ data: null, error: null }));
    const download = vi.fn(async () => ({
      data: new Blob([JSON.stringify(document)], { type: 'application/json' }),
      error: null,
    }));
    const storageFrom = vi.fn(() => ({ download }));
    const from = vi.fn((table: string) => {
      if (table === 'design_files') {
        return { select: sourceQuery.select, insert: fileInsert };
      }
      if (table === 'projects') return { select: projectQuery.select };
      if (table === 'design_file_versions') return { insert: versionInsert };
      throw new Error(`Unexpected table ${table}`);
    });

    await copyCloudFile({
      supabase: {
        from,
        storage: { from: storageFrom },
      } as never,
      userId: 'user-1',
      fileId: 'source-1',
    });

    expect(storageFrom).toHaveBeenCalledWith('openpencil-assets');
    expect(download).toHaveBeenCalledWith('user-1/source-1/documents/rev-4.json');
    expect(fileInsert).toHaveBeenCalledWith(expect.objectContaining({ document }));
    expect(versionInsert).toHaveBeenCalledWith(expect.objectContaining({ document }));
  });
});
