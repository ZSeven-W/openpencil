import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { H3Event } from 'h3';

const h3Mocks = vi.hoisted(() => ({
  body: {} as unknown,
  query: {} as Record<string, string>,
  params: {} as Record<string, string | undefined>,
  status: undefined as number | undefined,
}));

const cloudSupabaseMocks = vi.hoisted(() => ({
  getCloudSupabase: vi.fn(),
  toApiError: vi.fn((statusCode: number, code: string, message: string, details?: unknown) => {
    const error = new Error(message) as Error & {
      statusCode: number;
      statusMessage: string;
      data?: unknown;
    };
    error.statusCode = statusCode;
    error.statusMessage = message;
    error.data = { code, details };
    return error;
  }),
}));

const cloudDocumentStorageMocks = vi.hoisted(() => ({
  prepareCloudDocumentForStorage: vi.fn(async ({ document }: { document: unknown }) => ({
    storedDocument: document,
  })),
  resolveCloudDocumentFromStorage: vi.fn(async (_supabase: unknown, document: unknown) => document),
}));

vi.mock('h3', async () => {
  const actual = await vi.importActual<typeof import('h3')>('h3');
  return {
    ...actual,
    assertBodySize: vi.fn(async () => {}),
    defineEventHandler: (handler: unknown) => handler,
    readBody: vi.fn(async () => h3Mocks.body),
    getQuery: vi.fn(() => h3Mocks.query),
    getRouterParam: vi.fn((_event: unknown, name: string) => h3Mocks.params[name]),
    setResponseStatus: vi.fn((_event: unknown, status: number) => {
      h3Mocks.status = status;
    }),
  };
});

vi.mock('../../../utils/cloud-supabase', () => cloudSupabaseMocks);
vi.mock('../../../utils/cloud-document-storage', () => cloudDocumentStorageMocks);
vi.mock('../../../../server/utils/cloud-document-storage', () => cloudDocumentStorageMocks);
vi.mock('../../../utils/cloud-activity-events', () => ({
  recordCloudActivity: vi.fn(async () => {}),
}));

const now = '2026-05-12T08:00:00.000Z';
const user = { id: 'user-1', email: 'alice@example.com' };
const event = {} as H3Event;

const projectRow = {
  id: '11111111-1111-4111-8111-111111111111',
  name: 'Mobile App',
  description: null,
  icon: null,
  color: null,
  created_at: now,
  updated_at: now,
};

const folderRow = {
  id: '22222222-2222-4222-8222-222222222222',
  project_id: projectRow.id,
  parent_id: null,
  name: 'Flows',
  sort_order: 0,
  created_at: now,
  updated_at: now,
};

const fileRow = {
  id: '33333333-3333-4333-8333-333333333333',
  project_id: projectRow.id,
  folder_id: null,
  name: 'Home Screen',
  document: { version: '1.0.0', children: [] },
  thumbnail_path: null,
  revision: 1,
  metadata: {},
  starred: false,
  last_opened_at: null,
  deleted_at: null,
  created_at: now,
  updated_at: now,
};

const shareRow = {
  id: '44444444-4444-4444-8444-444444444444',
  file_id: fileRow.id,
  owner_id: user.id,
  shared_with_user_id: null,
  shared_with_email: 'bob@example.com',
  role: 'viewer',
  created_at: now,
  updated_at: now,
};

function createSelectQuery(data: unknown, error: unknown = null) {
  const query = {
    select: vi.fn(() => query),
    eq: vi.fn(() => query),
    is: vi.fn(() => query),
    not: vi.fn(() => query),
    in: vi.fn(() => query),
    ilike: vi.fn(() => query),
    order: vi.fn(() => query),
    limit: vi.fn(() => query),
    single: vi.fn(async () => ({ data, error })),
    maybeSingle: vi.fn(async () => ({ data, error })),
    then: (resolve: (value: { data: unknown; error: unknown }) => unknown, reject?: (reason: unknown) => unknown) =>
      Promise.resolve({ data, error }).then(resolve, reject),
  };
  return query as typeof query & {
    eq: ReturnType<typeof vi.fn>;
    is: ReturnType<typeof vi.fn>;
    in: ReturnType<typeof vi.fn>;
    then: PromiseLike<{ data: unknown; error: unknown }>['then'];
  };
}

function createMutationQuery(data: unknown = null, error: unknown = null) {
  const query = {
    select: vi.fn(() => query),
    eq: vi.fn(() => query),
    is: vi.fn(() => query),
    not: vi.fn(() => query),
    delete: vi.fn(() => query),
    single: vi.fn(async () => ({ data, error })),
    then: (resolve: (value: { data: unknown; error: unknown }) => unknown, reject?: (reason: unknown) => unknown) =>
      Promise.resolve({ data, error }).then(resolve, reject),
  };
  return query as typeof query & {
    eq: ReturnType<typeof vi.fn>;
    is: ReturnType<typeof vi.fn>;
    then: PromiseLike<{ data: unknown; error: unknown }>['then'];
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  h3Mocks.body = {};
  h3Mocks.query = {};
  h3Mocks.params = {};
  h3Mocks.status = undefined;
  cloudSupabaseMocks.getCloudSupabase.mockReset();
  cloudDocumentStorageMocks.prepareCloudDocumentForStorage.mockClear();
  cloudDocumentStorageMocks.resolveCloudDocumentFromStorage.mockClear();
});

describe('cloud project routes', () => {
  it('creates a project with a 201 response', async () => {
    const insertQuery = createMutationQuery(projectRow);
    const insert = vi.fn(() => insertQuery);
    const supabase = { from: vi.fn(() => ({ insert })) };
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase, user });
    h3Mocks.body = { name: 'Mobile App' };

    const handler = (await import('../projects/index.post')).default;
    const result = await handler(event);

    expect(h3Mocks.status).toBe(201);
    expect(insert).toHaveBeenCalledWith({
      owner_id: 'user-1',
      name: 'Mobile App',
      description: null,
      icon: null,
      color: null,
    });
    expect(result.data).toMatchObject({ id: projectRow.id, name: 'Mobile App' });
  });

  it('renames a project owned by the user', async () => {
    const updateQuery = createMutationQuery({ ...projectRow, name: 'Renamed' });
    const update = vi.fn(() => updateQuery);
    const supabase = { from: vi.fn(() => ({ update })) };
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase, user });
    h3Mocks.params = { id: projectRow.id };
    h3Mocks.body = { name: 'Renamed' };

    const handler = (await import('../projects/[id].patch')).default;
    const result = await handler(event);

    expect(update).toHaveBeenCalledWith({ name: 'Renamed' });
    expect(updateQuery.eq).toHaveBeenCalledWith('id', projectRow.id);
    expect(updateQuery.eq).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(updateQuery.is).toHaveBeenCalledWith('deleted_at', null);
    expect(result.data.name).toBe('Renamed');
  });

  it('soft deletes a project and moves files to the fallback project', async () => {
    const existingQuery = createSelectQuery(projectRow);
    const deleteProjectQuery = createMutationQuery();
    const foldersQuery = createMutationQuery();
    const defaultProjectQuery = createSelectQuery({ ...projectRow, id: 'fallback-project' });
    const attachQuery = createMutationQuery();
    const moveFilesQuery = createMutationQuery();
    const update = vi
      .fn()
      .mockReturnValueOnce(deleteProjectQuery)
      .mockReturnValueOnce(foldersQuery)
      .mockReturnValueOnce(attachQuery)
      .mockReturnValueOnce(moveFilesQuery);
    const from = vi.fn((table: string) => {
      if (table === 'projects') return { select: existingQuery.select, update };
      if (table === 'folders') return { update };
      if (table === 'design_files') return { update };
      throw new Error(`Unexpected table ${table}`);
    });
    existingQuery.single.mockResolvedValueOnce({ data: projectRow, error: null });
    existingQuery.maybeSingle = defaultProjectQuery.maybeSingle;
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.params = { id: projectRow.id };

    const handler = (await import('../projects/[id].delete')).default;
    await expect(handler(event)).resolves.toBeNull();

    expect(h3Mocks.status).toBe(204);
    expect(update).toHaveBeenCalledWith(expect.objectContaining({ deleted_at: expect.any(String) }));
    expect(moveFilesQuery.eq).toHaveBeenCalledWith('project_id', projectRow.id);
  });
});

describe('cloud folder routes', () => {
  it('lists folders for the requested project and parent', async () => {
    const projectQuery = createSelectQuery({ id: projectRow.id });
    const folderListQuery = createSelectQuery([folderRow]);
    const from = vi.fn((table: string) => {
      if (table === 'projects') return { select: projectQuery.select };
      if (table === 'folders') return { select: folderListQuery.select };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.query = { projectId: projectRow.id, parentId: 'null' };

    const handler = (await import('../folders/index.get')).default;
    const result = await handler(event);

    expect(folderListQuery.eq).toHaveBeenCalledWith('project_id', projectRow.id);
    expect(folderListQuery.is).toHaveBeenCalledWith('parent_id', null);
    expect(result.data).toEqual([expect.objectContaining({ id: folderRow.id, name: 'Flows' })]);
  });

  it('creates a folder in a project', async () => {
    const defaultProjectQuery = createSelectQuery(projectRow);
    const attachQuery = createMutationQuery();
    const folderInsertQuery = createMutationQuery(folderRow);
    const insert = vi.fn(() => folderInsertQuery);
    const update = vi.fn(() => attachQuery);
    const from = vi.fn((table: string) => {
      if (table === 'projects') return { select: defaultProjectQuery.select };
      if (table === 'design_files') return { update };
      if (table === 'folders') return { insert };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.body = { name: 'Flows' };

    const handler = (await import('../folders/index.post')).default;
    const result = await handler(event);

    expect(h3Mocks.status).toBe(201);
    expect(insert).toHaveBeenCalledWith({
      owner_id: 'user-1',
      project_id: projectRow.id,
      parent_id: null,
      name: 'Flows',
      sort_order: 0,
    });
    expect(result.data.name).toBe('Flows');
  });

  it('updates and soft deletes folders', async () => {
    const currentQuery = createSelectQuery(folderRow);
    const projectQuery = createSelectQuery({ id: projectRow.id });
    const updateQuery = createMutationQuery({ ...folderRow, name: 'Updated' });
    const update = vi.fn(() => updateQuery);
    const fromForPatch = vi.fn((table: string) => {
      if (table === 'folders') return { select: currentQuery.select, update };
      if (table === 'projects') return { select: projectQuery.select };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValueOnce({
      supabase: { from: fromForPatch },
      user,
    });
    h3Mocks.params = { id: folderRow.id };
    h3Mocks.body = { name: 'Updated' };

    const patchHandler = (await import('../folders/[id].patch')).default;
    const patchResult = await patchHandler(event);
    expect(update).toHaveBeenCalledWith({ name: 'Updated' });
    expect(patchResult.data.name).toBe('Updated');

    const existingQuery = createSelectQuery({ id: folderRow.id });
    const deleteQuery = createMutationQuery();
    const childQuery = createMutationQuery();
    const filesQuery = createMutationQuery();
    const deleteUpdate = vi
      .fn()
      .mockReturnValueOnce(deleteQuery)
      .mockReturnValueOnce(childQuery)
      .mockReturnValueOnce(filesQuery);
    const fromForDelete = vi.fn((table: string) => {
      if (table === 'folders') return { select: existingQuery.select, update: deleteUpdate };
      if (table === 'design_files') return { update: deleteUpdate };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValueOnce({
      supabase: { from: fromForDelete },
      user,
    });
    h3Mocks.params = { id: folderRow.id };

    const deleteHandler = (await import('../folders/[id].delete')).default;
    await expect(deleteHandler(event)).resolves.toBeNull();
    expect(h3Mocks.status).toBe(204);
    expect(deleteUpdate).toHaveBeenCalledWith(expect.objectContaining({ deleted_at: expect.any(String) }));
    expect(filesQuery.eq).toHaveBeenCalledWith('folder_id', folderRow.id);
  });
});

describe('cloud file routes', () => {
  it('lists files with search, sort, folder, project, and non-trash filters', async () => {
    const projectQuery = createSelectQuery({ id: projectRow.id });
    const folderQuery = createSelectQuery({ id: folderRow.id });
    const fileListQuery = createSelectQuery([fileRow]);
    const from = vi.fn((table: string) => {
      if (table === 'projects') return { select: projectQuery.select };
      if (table === 'folders') return { select: folderQuery.select };
      if (table === 'design_files') return { select: fileListQuery.select };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.query = {
      projectId: projectRow.id,
      folderId: folderRow.id,
      search: 'Home',
      sort: 'name_asc',
    };

    const handler = (await import('../files/index.get')).default;
    const result = await handler(event);

    expect(fileListQuery.eq).toHaveBeenCalledWith('project_id', projectRow.id);
    expect(fileListQuery.eq).toHaveBeenCalledWith('folder_id', folderRow.id);
    expect(fileListQuery.is).toHaveBeenCalledWith('deleted_at', null);
    expect(fileListQuery.ilike).toHaveBeenCalledWith('name', '%Home%');
    expect(fileListQuery.order).toHaveBeenCalledWith('name', { ascending: true });
    expect(result.data).toEqual([expect.objectContaining({ id: fileRow.id, name: 'Home Screen' })]);
  });

  it('soft deletes a file with a 204 response', async () => {
    const deleteQuery = createMutationQuery({ id: fileRow.id });
    const update = vi.fn(() => deleteQuery);
    const supabase = { from: vi.fn(() => ({ update })) };
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase, user });
    h3Mocks.params = { id: fileRow.id };

    const handler = (await import('../files/[id].delete')).default;
    await expect(handler(event)).resolves.toBeNull();

    expect(h3Mocks.status).toBe(204);
    expect(update).toHaveBeenCalledWith({ deleted_at: expect.any(String) });
    expect(deleteQuery.eq).toHaveBeenCalledWith('id', fileRow.id);
    expect(deleteQuery.eq).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(deleteQuery.is).toHaveBeenCalledWith('deleted_at', null);
  });

  it('rejects stale document saves with structured revision conflict details', async () => {
    const accessQuery = createSelectQuery({ id: fileRow.id, owner_id: user.id });
    const currentQuery = createSelectQuery({ ...fileRow, revision: 9 });
    const from = vi
      .fn()
      .mockReturnValueOnce({ select: accessQuery.select })
      .mockReturnValueOnce({ select: currentQuery.select });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.params = { id: fileRow.id };
    h3Mocks.body = {
      document: { version: '1.0.0', children: [] },
      expectedRevision: 7,
    };

    const handler = (await import('../files/[id].patch')).default;

    await expect(handler(event)).rejects.toMatchObject({
      statusCode: 409,
      data: {
        code: 'revision_conflict',
        details: {
          fileId: fileRow.id,
          expectedRevision: 7,
          serverRevision: 9,
        },
      },
    });
  });

  it('saves a document only when expectedRevision still matches', async () => {
    const accessQuery = createSelectQuery({ id: fileRow.id, owner_id: user.id });
    const currentQuery = createSelectQuery({ ...fileRow, revision: 7 });
    const updateQuery = createMutationQuery({ ...fileRow, revision: 8 });
    const versionQuery = createMutationQuery();
    const update = vi.fn(() => updateQuery);
    const insert = vi.fn(() => versionQuery);
    const designFilesSelect = vi
      .fn()
      .mockReturnValueOnce(accessQuery)
      .mockReturnValueOnce(currentQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') {
        return {
          select: designFilesSelect,
          update,
        };
      }
      if (table === 'design_file_versions') return { insert };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.params = { id: fileRow.id };
    h3Mocks.body = {
      document: { version: '1.0.0', children: [] },
      expectedRevision: 7,
      source: 'manual_save',
      label: 'Manual save',
    };
    const body = h3Mocks.body as { document: unknown };

    const handler = (await import('../files/[id].patch')).default;
    const result = await handler(event);

    expect(update).toHaveBeenCalledWith(
      expect.objectContaining({
        revision: 8,
        document: body.document,
      }),
    );
    expect(updateQuery.eq).toHaveBeenCalledWith('revision', 7);
    expect(insert).toHaveBeenCalledWith(
      expect.objectContaining({
        file_id: fileRow.id,
        revision: 8,
        source: 'manual_save',
        label: 'Manual save',
      }),
    );
    expect(result.data).toMatchObject({ id: fileRow.id, revision: 8 });
  });

  it('lists files shared with the current user', async () => {
    const byUserSharesQuery = createSelectQuery([]);
    const byEmailSharesQuery = createSelectQuery([shareRow]);
    const fileListQuery = createSelectQuery([fileRow]);
    const from = vi
      .fn()
      .mockReturnValueOnce({ select: byUserSharesQuery.select })
      .mockReturnValueOnce({ select: byEmailSharesQuery.select })
      .mockReturnValueOnce({ select: fileListQuery.select });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.query = { view: 'shared', search: 'Home', sort: 'name_desc' };

    const handler = (await import('../files/index.get')).default;
    const result = await handler(event);

    expect(byUserSharesQuery.eq).toHaveBeenCalledWith('shared_with_user_id', user.id);
    expect(byEmailSharesQuery.ilike).toHaveBeenCalledWith('shared_with_email', user.email);
    expect(fileListQuery.in).toHaveBeenCalledWith('id', [fileRow.id]);
    expect(fileListQuery.ilike).toHaveBeenCalledWith('name', '%Home%');
    expect(fileListQuery.order).toHaveBeenCalledWith('name', { ascending: false });
    expect(result.data).toEqual([
      expect.objectContaining({ id: fileRow.id, name: 'Home Screen', shareRole: 'viewer' }),
    ]);
  });

  it('copies a file into the requested folder and creates the copied version', async () => {
    const sourceQuery = createSelectQuery(fileRow);
    const projectQuery = createSelectQuery({ id: projectRow.id });
    const folderQuery = createSelectQuery({ id: folderRow.id });
    const copiedQuery = createMutationQuery({ ...fileRow, id: 'copy-file', folder_id: folderRow.id });
    const versionQuery = createMutationQuery();
    const insert = vi.fn().mockReturnValueOnce(copiedQuery).mockReturnValueOnce(versionQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') return { select: sourceQuery.select, insert };
      if (table === 'projects') return { select: projectQuery.select };
      if (table === 'folders') return { select: folderQuery.select };
      if (table === 'design_file_versions') return { insert };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.params = { id: fileRow.id };
    h3Mocks.body = { name: 'Copy', projectId: projectRow.id, folderId: folderRow.id };

    const handler = (await import('../files/[id]/copy.post')).default;
    const result = await handler(event);

    expect(h3Mocks.status).toBe(201);
    expect(sourceQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(folderQuery.eq).toHaveBeenCalledWith('project_id', projectRow.id);
    expect(insert).toHaveBeenCalledWith(
      expect.objectContaining({
        owner_id: user.id,
        project_id: projectRow.id,
        folder_id: folderRow.id,
        name: 'Copy',
        revision: 1,
      }),
    );
    expect(insert).toHaveBeenCalledWith(
      expect.objectContaining({
        file_id: 'copy-file',
        owner_id: user.id,
        revision: 1,
        label: 'Copied version',
      }),
    );
    expect(result.data).toMatchObject({ id: 'copy-file', folderId: folderRow.id });
  });

  it('restores a selected version with optimistic revision protection and creates a restore snapshot', async () => {
    const versionId = '55555555-5555-4555-8555-555555555555';
    const versionQuery = createSelectQuery({ document: { version: '1.0.0', children: [] } });
    const currentQuery = createSelectQuery({ revision: 5 });
    const updateQuery = createMutationQuery({ ...fileRow, revision: 6 });
    const snapshotQuery = createMutationQuery();
    const update = vi.fn(() => updateQuery);
    const insert = vi.fn(() => snapshotQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_file_versions') {
        return { select: versionQuery.select, insert };
      }
      if (table === 'design_files') {
        return { select: currentQuery.select, update };
      }
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.params = { id: fileRow.id };
    h3Mocks.body = { versionId };

    const handler = (await import('../files/[id]/restore-version.post')).default;
    const result = await handler(event);

    expect(versionQuery.eq).toHaveBeenCalledWith('id', versionId);
    expect(versionQuery.eq).toHaveBeenCalledWith('file_id', fileRow.id);
    expect(versionQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(update).toHaveBeenCalledWith(
      expect.objectContaining({
        revision: 6,
        document: { version: '1.0.0', children: [] },
      }),
    );
    expect(updateQuery.eq).toHaveBeenCalledWith('revision', 5);
    expect(insert).toHaveBeenCalledWith(
      expect.objectContaining({
        file_id: fileRow.id,
        owner_id: user.id,
        revision: 6,
        source: 'restore',
        label: `Restored ${versionId}`,
      }),
    );
    expect(result.data).toMatchObject({ id: fileRow.id, revision: 6 });
  });

  it('restores and permanently deletes trashed files only', async () => {
    const trashedRow = { ...fileRow, deleted_at: now };
    const restoreQuery = createMutationQuery({ ...fileRow, deleted_at: null });
    const restoreUpdate = vi.fn(() => restoreQuery);
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValueOnce({
      supabase: { from: vi.fn(() => ({ update: restoreUpdate })) },
      user,
    });
    h3Mocks.params = { id: fileRow.id };

    const restoreHandler = (await import('../files/[id]/restore.post')).default;
    const restoreResult = await restoreHandler(event);

    expect(restoreUpdate).toHaveBeenCalledWith({ deleted_at: null });
    expect(restoreQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(restoreQuery.not).toHaveBeenCalledWith('deleted_at', 'is', null);
    expect(restoreResult.data).toMatchObject({ id: fileRow.id, deletedAt: null });

    const lookupQuery = createSelectQuery({ id: trashedRow.id });
    const deleteQuery = createMutationQuery();
    const deleteFn = vi.fn(() => deleteQuery);
    const fromForPermanentDelete = vi.fn((table: string) => {
      if (table !== 'design_files') throw new Error(`Unexpected table ${table}`);
      return { select: lookupQuery.select, delete: deleteFn };
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValueOnce({
      supabase: { from: fromForPermanentDelete },
      user,
    });
    h3Mocks.params = { id: fileRow.id };

    const permanentDeleteHandler = (await import('../files/[id]/permanent-delete.delete')).default;
    await expect(permanentDeleteHandler(event)).resolves.toBeNull();

    expect(h3Mocks.status).toBe(204);
    expect(lookupQuery.not).toHaveBeenCalledWith('deleted_at', 'is', null);
    expect(deleteFn).toHaveBeenCalled();
    expect(deleteQuery.eq).toHaveBeenCalledWith('id', fileRow.id);
    expect(deleteQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
  });
});

describe('cloud file sharing routes', () => {
  it('lists active shares for an owned file', async () => {
    const ownedQuery = createSelectQuery({ id: fileRow.id });
    const sharesQuery = createSelectQuery([shareRow]);
    const from = vi
      .fn()
      .mockReturnValueOnce({ select: ownedQuery.select })
      .mockReturnValueOnce({ select: sharesQuery.select });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.params = { id: fileRow.id };

    const handler = (await import('../files/[id]/shares/index.get')).default;
    const result = await handler(event);

    expect(ownedQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(sharesQuery.eq).toHaveBeenCalledWith('file_id', fileRow.id);
    expect(sharesQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(sharesQuery.is).toHaveBeenCalledWith('revoked_at', null);
    expect(result.data).toEqual([
      expect.objectContaining({ id: shareRow.id, sharedWithEmail: 'bob@example.com' }),
    ]);
  });

  it('creates and revokes file shares', async () => {
    const ownedQuery = createSelectQuery({ id: fileRow.id });
    const createQuery = createMutationQuery(shareRow);
    const insert = vi.fn(() => createQuery);
    const fromForCreate = vi.fn((table: string) => {
      if (table === 'design_files') return { select: ownedQuery.select };
      if (table === 'design_file_shares') return { insert };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValueOnce({
      supabase: { from: fromForCreate },
      user,
    });
    h3Mocks.params = { id: fileRow.id };
    h3Mocks.body = { email: 'BOB@example.com', role: 'viewer' };

    const createHandler = (await import('../files/[id]/shares/index.post')).default;
    const createResult = await createHandler(event);

    expect(h3Mocks.status).toBe(201);
    expect(insert).toHaveBeenCalledWith({
      file_id: fileRow.id,
      owner_id: user.id,
      shared_with_user_id: null,
      shared_with_email: 'bob@example.com',
      role: 'viewer',
    });
    expect(createResult.data).toMatchObject({ id: shareRow.id, sharedWithEmail: 'bob@example.com' });

    const revokeQuery = createMutationQuery({ id: shareRow.id });
    const update = vi.fn(() => revokeQuery);
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValueOnce({
      supabase: { from: vi.fn(() => ({ update })) },
      user,
    });
    h3Mocks.params = { id: fileRow.id, shareId: shareRow.id };

    const revokeHandler = (await import('../files/[id]/shares/[shareId].delete')).default;
    await expect(revokeHandler(event)).resolves.toBeNull();

    expect(h3Mocks.status).toBe(204);
    expect(update).toHaveBeenCalledWith({ revoked_at: expect.any(String) });
    expect(revokeQuery.eq).toHaveBeenCalledWith('file_id', fileRow.id);
    expect(revokeQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(revokeQuery.is).toHaveBeenCalledWith('revoked_at', null);
  });
});
