import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { H3Event } from 'h3';

const h3Mocks = vi.hoisted(() => ({
  body: {} as unknown,
  query: {} as Record<string, string>,
  params: {} as Record<string, string | undefined>,
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
  };
});

vi.mock('../../../utils/cloud-supabase', () => cloudSupabaseMocks);
vi.mock('../../../utils/cloud-document-storage', () => cloudDocumentStorageMocks);
vi.mock('../../../../server/utils/cloud-document-storage', () => cloudDocumentStorageMocks);

const event = {} as H3Event;
const now = '2026-05-12T08:00:00.000Z';
const user = { id: 'user-1', email: 'alice@example.com' };
const owner = { id: 'owner-1', email: 'owner@example.com' };
const editor = { id: 'editor-1', email: 'editor@example.com' };
const viewer = { id: 'viewer-1', email: 'viewer@example.com' };
const fileRow = {
  id: '33333333-3333-4333-8333-333333333333',
  project_id: '11111111-1111-4111-8111-111111111111',
  folder_id: null,
  name: 'Home Screen',
  document: { version: '1.0.0', children: [] },
  thumbnail_path: null,
  revision: 9,
  metadata: {},
  starred: false,
  last_opened_at: null,
  deleted_at: null,
  created_at: now,
  updated_at: now,
};

function createSelectQuery(data: unknown, error: unknown = null) {
  const query = {
    select: vi.fn(() => query),
    eq: vi.fn(() => query),
    is: vi.fn(() => query),
    in: vi.fn(() => query),
    ilike: vi.fn(() => query),
    lt: vi.fn(() => query),
    order: vi.fn(() => query),
    limit: vi.fn(() => query),
    range: vi.fn(() => query),
    single: vi.fn(async () => ({ data, error })),
    maybeSingle: vi.fn(async () => ({ data, error })),
    then: (
      resolve: (value: { data: unknown; error: unknown }) => unknown,
      reject?: (reason: unknown) => unknown,
    ) => Promise.resolve({ data, error }).then(resolve, reject),
  };
  return query;
}

function createMutationQuery(data: unknown = null, error: unknown = null) {
  const query = {
    select: vi.fn(() => query),
    eq: vi.fn(() => query),
    is: vi.fn(() => query),
    in: vi.fn(() => query),
    delete: vi.fn(() => query),
    single: vi.fn(async () => ({ data, error })),
    then: (
      resolve: (value: { data: unknown; error: unknown }) => unknown,
      reject?: (reason: unknown) => unknown,
    ) => Promise.resolve({ data, error }).then(resolve, reject),
  };
  return query;
}

function createSharedAccessQueries(role: 'viewer' | 'editor') {
  return {
    ownedQuery: createSelectQuery(null),
    shareByUserQuery: createSelectQuery({ role }),
    fileOwnerQuery: createSelectQuery({ owner_id: owner.id }),
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  h3Mocks.body = {};
  h3Mocks.query = {};
  h3Mocks.params = {};
  cloudSupabaseMocks.getCloudSupabase.mockReset();
});

describe('cloud version and activity routes', () => {
  it('force saves a document over a newer remote revision and records activity', async () => {
    const accessQuery = createSelectQuery({ id: fileRow.id, owner_id: user.id });
    const currentQuery = createSelectQuery(fileRow);
    const updateQuery = createMutationQuery({ ...fileRow, revision: 10 });
    const versionQuery = createMutationQuery();
    const activityQuery = createMutationQuery();
    const update = vi.fn(() => updateQuery);
    const insert = vi.fn().mockReturnValueOnce(versionQuery).mockReturnValueOnce(activityQuery);
    const designFilesSelect = vi.fn().mockReturnValueOnce(accessQuery).mockReturnValueOnce(currentQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') return { select: designFilesSelect, update };
      if (table === 'design_file_versions') return { insert };
      if (table === 'activity_events') return { insert };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.params = { id: fileRow.id };
    h3Mocks.body = {
      document: { version: '1.0.0', children: [{ id: 'local' }] },
      expectedRevision: 7,
      force: true,
      label: 'Forced overwrite',
    };

    const handler = (await import('../files/[id].patch')).default;
    const result = await handler(event);

    expect(update).toHaveBeenCalledWith(expect.objectContaining({ revision: 10 }));
    expect(currentQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(updateQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(updateQuery.eq).not.toHaveBeenCalledWith('revision', 7);
    expect(insert).toHaveBeenCalledWith(
      expect.objectContaining({
        file_id: fileRow.id,
        actor_id: user.id,
        revision: 10,
        label: 'Forced overwrite',
        size_bytes: expect.any(Number),
      }),
    );
    expect(insert).toHaveBeenCalledWith(
      expect.objectContaining({
        owner_id: user.id,
        actor_id: user.id,
        file_id: fileRow.id,
        type: 'file_force_saved',
      }),
    );
    expect(result.data.revision).toBe(10);
  });

  it('prunes old autosave versions after an autosave snapshot', async () => {
    const accessQuery = createSelectQuery({ id: fileRow.id, owner_id: user.id });
    const currentQuery = createSelectQuery({ ...fileRow, revision: 9 });
    const updateQuery = createMutationQuery({ ...fileRow, revision: 10 });
    const versionQuery = createMutationQuery();
    const staleAutosavesQuery = createSelectQuery([{ id: 'old-version-1' }, { id: 'old-version-2' }]);
    const deleteOldAutosavesQuery = createMutationQuery();
    const activityQuery = createMutationQuery();
    const update = vi.fn(() => updateQuery);
    const insertVersion = vi.fn(() => versionQuery);
    const deleteOldAutosaves = vi.fn(() => deleteOldAutosavesQuery);
    const designFilesSelect = vi.fn().mockReturnValueOnce(accessQuery).mockReturnValueOnce(currentQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') return { select: designFilesSelect, update };
      if (table === 'design_file_versions') {
        return {
          insert: insertVersion,
          select: staleAutosavesQuery.select,
          delete: deleteOldAutosaves,
        };
      }
      if (table === 'activity_events') return { insert: vi.fn(() => activityQuery) };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.params = { id: fileRow.id };
    h3Mocks.body = {
      document: { version: '1.0.0', children: [{ id: 'autosave' }] },
      expectedRevision: 9,
      source: 'autosave',
      label: 'Autosave',
    };

    const handler = (await import('../files/[id].patch')).default;
    const result = await handler(event);

    expect(insertVersion).toHaveBeenCalledWith(expect.objectContaining({ source: 'autosave' }));
    expect(staleAutosavesQuery.eq).toHaveBeenCalledWith('file_id', fileRow.id);
    expect(staleAutosavesQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(staleAutosavesQuery.eq).toHaveBeenCalledWith('source', 'autosave');
    expect(staleAutosavesQuery.range).toHaveBeenCalledWith(20, 1000);
    expect(deleteOldAutosaves).toHaveBeenCalled();
    expect(deleteOldAutosavesQuery.in).toHaveBeenCalledWith('id', [
      'old-version-1',
      'old-version-2',
    ]);
    expect(deleteOldAutosavesQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(result.data.revision).toBe(10);
  });

  it('updates a version label and records a version_labeled activity event', async () => {
    const ownedQuery = createSelectQuery({ id: fileRow.id });
    const updateQuery = createMutationQuery({
      id: 'version-1',
      file_id: fileRow.id,
      revision: 4,
      source: 'manual_save',
      label: 'Approved',
      actor_id: user.id,
      size_bytes: 2048,
      created_at: now,
    });
    const activityQuery = createMutationQuery();
    const update = vi.fn(() => updateQuery);
    const insert = vi.fn(() => activityQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') return { select: ownedQuery.select };
      if (table === 'design_file_versions') return { update };
      if (table === 'activity_events') return { insert };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.params = { id: fileRow.id, versionId: 'version-1' };
    h3Mocks.body = { label: 'Approved' };

    const handler = (await import('../files/[id]/versions/[versionId].patch')).default;
    const result = await handler(event);

    expect(update).toHaveBeenCalledWith({ label: 'Approved' });
    expect(insert).toHaveBeenCalledWith(
      expect.objectContaining({
        owner_id: user.id,
        actor_id: user.id,
        file_id: fileRow.id,
        type: 'version_labeled',
        metadata: expect.objectContaining({ label: 'Approved', revision: 4 }),
      }),
    );
    expect(result.data).toMatchObject({ id: 'version-1', label: 'Approved', sizeBytes: 2048 });
  });

  it('lets shared viewers read version history with owner-scoped version rows', async () => {
    const access = createSharedAccessQueries('viewer');
    const versionsQuery = createSelectQuery([
      {
        id: 'version-1',
        file_id: fileRow.id,
        revision: 9,
        source: 'manual_save',
        label: 'Review',
        actor_id: owner.id,
        size_bytes: 2048,
        created_at: now,
      },
    ]);
    const designFilesSelect = vi
      .fn()
      .mockReturnValueOnce(access.ownedQuery)
      .mockReturnValueOnce(access.fileOwnerQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') {
        return { select: designFilesSelect };
      }
      if (table === 'design_file_shares') {
        return {
          select: vi.fn().mockReturnValueOnce(access.shareByUserQuery),
        };
      }
      if (table === 'design_file_versions') return { select: versionsQuery.select };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user: viewer });
    h3Mocks.params = { id: fileRow.id };

    const handler = (await import('../files/[id]/versions.get')).default;
    const result = await handler(event);

    expect(access.shareByUserQuery.eq).toHaveBeenCalledWith('shared_with_user_id', viewer.id);
    expect(versionsQuery.eq).toHaveBeenCalledWith('file_id', fileRow.id);
    expect(versionsQuery.eq).toHaveBeenCalledWith('owner_id', owner.id);
    expect(result.data).toEqual([
      expect.objectContaining({ id: 'version-1', actorId: owner.id, sizeBytes: 2048 }),
    ]);
  });

  it('rejects shared viewers when editing version labels', async () => {
    const access = createSharedAccessQueries('viewer');
    const update = vi.fn();
    const designFilesSelect = vi
      .fn()
      .mockReturnValueOnce(access.ownedQuery)
      .mockReturnValueOnce(access.fileOwnerQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') {
        return { select: designFilesSelect };
      }
      if (table === 'design_file_shares') {
        return {
          select: vi.fn().mockReturnValueOnce(access.shareByUserQuery),
        };
      }
      if (table === 'design_file_versions') return { update };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user: viewer });
    h3Mocks.params = { id: fileRow.id, versionId: 'version-1' };
    h3Mocks.body = { label: 'Viewer label' };

    const handler = (await import('../files/[id]/versions/[versionId].patch')).default;

    await expect(handler(event)).rejects.toMatchObject({
      statusCode: 404,
      statusMessage: 'Cloud file not found',
    });
    expect(update).not.toHaveBeenCalled();
  });

  it('lets shared editors edit labels against the owner version row', async () => {
    const access = createSharedAccessQueries('editor');
    const updateQuery = createMutationQuery({
      id: 'version-1',
      file_id: fileRow.id,
      revision: 4,
      source: 'manual_save',
      label: 'Editor approved',
      actor_id: editor.id,
      size_bytes: 2048,
      created_at: now,
    });
    const activityQuery = createMutationQuery();
    const update = vi.fn(() => updateQuery);
    const insert = vi.fn(() => activityQuery);
    const designFilesSelect = vi
      .fn()
      .mockReturnValueOnce(access.ownedQuery)
      .mockReturnValueOnce(access.fileOwnerQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') {
        return { select: designFilesSelect };
      }
      if (table === 'design_file_shares') {
        return {
          select: vi.fn().mockReturnValueOnce(access.shareByUserQuery),
        };
      }
      if (table === 'design_file_versions') return { update };
      if (table === 'activity_events') return { insert };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user: editor });
    h3Mocks.params = { id: fileRow.id, versionId: 'version-1' };
    h3Mocks.body = { label: 'Editor approved' };

    const handler = (await import('../files/[id]/versions/[versionId].patch')).default;
    const result = await handler(event);

    expect(update).toHaveBeenCalledWith({ label: 'Editor approved' });
    expect(updateQuery.eq).toHaveBeenCalledWith('owner_id', owner.id);
    expect(insert).toHaveBeenCalledWith(
      expect.objectContaining({
        owner_id: owner.id,
        actor_id: editor.id,
        file_id: fileRow.id,
        type: 'version_labeled',
      }),
    );
    expect(result.data).toMatchObject({ id: 'version-1', label: 'Editor approved' });
  });

  it('lists recent activity for an owned cloud file', async () => {
    const ownedQuery = createSelectQuery({ id: fileRow.id });
    const activityQuery = createSelectQuery([
      {
        id: 'activity-1',
        file_id: fileRow.id,
        generation_id: null,
        actor_id: user.id,
        owner_id: user.id,
        type: 'file_saved',
        metadata: { revision: 9 },
        created_at: now,
      },
    ]);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') return { select: ownedQuery.select };
      if (table === 'activity_events') return { select: activityQuery.select };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.params = { id: fileRow.id };

    const handler = (await import('../files/[id]/activity.get')).default;
    const result = await handler(event);

    expect(activityQuery.eq).toHaveBeenCalledWith('file_id', fileRow.id);
    expect(activityQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(result.meta).toEqual({ nextCursor: null, limit: 20 });
    expect(result.data).toEqual([
      expect.objectContaining({ id: 'activity-1', type: 'file_saved', actorId: user.id }),
    ]);
  });

  it('filters and paginates shared activity with an exclusive next cursor', async () => {
    const access = createSharedAccessQueries('viewer');
    const activityQuery = createSelectQuery([
      {
        id: 'activity-1',
        file_id: fileRow.id,
        generation_id: null,
        actor_id: owner.id,
        owner_id: owner.id,
        type: 'file_shared',
        metadata: { role: 'viewer' },
        created_at: '2026-05-12T08:01:00.000Z',
      },
      {
        id: 'activity-2',
        file_id: fileRow.id,
        generation_id: null,
        actor_id: owner.id,
        owner_id: owner.id,
        type: 'file_shared',
        metadata: { role: 'editor' },
        created_at: '2026-05-12T08:00:00.000Z',
      },
      {
        id: 'activity-3',
        file_id: fileRow.id,
        generation_id: null,
        actor_id: owner.id,
        owner_id: owner.id,
        type: 'file_shared',
        metadata: {},
        created_at: '2026-05-12T07:59:00.000Z',
      },
    ]);
    const designFilesSelect = vi
      .fn()
      .mockReturnValueOnce(access.ownedQuery)
      .mockReturnValueOnce(access.fileOwnerQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') {
        return { select: designFilesSelect };
      }
      if (table === 'design_file_shares') {
        return {
          select: vi.fn().mockReturnValueOnce(access.shareByUserQuery),
        };
      }
      if (table === 'activity_events') return { select: activityQuery.select };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user: viewer });
    h3Mocks.params = { id: fileRow.id };
    h3Mocks.query = {
      type: 'file_shared',
      cursor: '2026-05-12T08:02:00.000Z',
      limit: '2',
    };

    const handler = (await import('../files/[id]/activity.get')).default;
    const result = await handler(event);

    expect(access.shareByUserQuery.eq).toHaveBeenCalledWith('shared_with_user_id', viewer.id);
    expect(activityQuery.eq).toHaveBeenCalledWith('file_id', fileRow.id);
    expect(activityQuery.eq).toHaveBeenCalledWith('owner_id', owner.id);
    expect(activityQuery.eq).toHaveBeenCalledWith('type', 'file_shared');
    expect(activityQuery.lt).toHaveBeenCalledWith('created_at', '2026-05-12T08:02:00.000Z');
    expect(activityQuery.limit).toHaveBeenCalledWith(3);
    expect(result.data).toEqual([
      expect.objectContaining({ id: 'activity-1', ownerId: owner.id }),
      expect.objectContaining({ id: 'activity-2', ownerId: owner.id }),
    ]);
    expect(result.meta).toEqual({ nextCursor: '2026-05-12T08:00:00.000Z', limit: 2 });
  });

  it('rejects invalid activity filters before querying activity events', async () => {
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({
      supabase: { from: vi.fn() },
      user,
    });
    h3Mocks.params = { id: fileRow.id };
    h3Mocks.query = { type: 'unknown_event' };

    const handler = (await import('../files/[id]/activity.get')).default;

    await expect(handler(event)).rejects.toMatchObject({
      statusCode: 400,
      data: { code: 'validation_error' },
    });
    expect(cloudSupabaseMocks.getCloudSupabase).not.toHaveBeenCalled();
  });

  it('lets shared editors restore versions with owner-scoped file updates and audit entries', async () => {
    const access = createSharedAccessQueries('editor');
    const versionId = '55555555-5555-4555-8555-555555555555';
    const versionQuery = createSelectQuery({ document: { version: '1.0.0', children: [] } });
    const currentQuery = createSelectQuery({ revision: 9 });
    const updateQuery = createMutationQuery({ ...fileRow, owner_id: owner.id, revision: 10 });
    const snapshotQuery = createMutationQuery();
    const activityQuery = createMutationQuery();
    const update = vi.fn(() => updateQuery);
    const versionInsert = vi.fn(() => snapshotQuery);
    const activityInsert = vi.fn(() => activityQuery);
    const designFilesSelect = vi
      .fn()
      .mockReturnValueOnce(access.ownedQuery)
      .mockReturnValueOnce(access.fileOwnerQuery)
      .mockReturnValueOnce(currentQuery);
    const from = vi.fn((table: string) => {
      if (table === 'design_files') {
        return { select: designFilesSelect, update };
      }
      if (table === 'design_file_shares') {
        return {
          select: vi.fn().mockReturnValueOnce(access.shareByUserQuery),
        };
      }
      if (table === 'design_file_versions') {
        return { select: versionQuery.select, insert: versionInsert };
      }
      if (table === 'activity_events') return { insert: activityInsert };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user: editor });
    h3Mocks.params = { id: fileRow.id };
    h3Mocks.body = { versionId };

    const handler = (await import('../files/[id]/restore-version.post')).default;
    const result = await handler(event);

    expect(versionQuery.eq).toHaveBeenCalledWith('owner_id', owner.id);
    expect(currentQuery.eq).toHaveBeenCalledWith('owner_id', owner.id);
    expect(updateQuery.eq).toHaveBeenCalledWith('owner_id', owner.id);
    expect(versionInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        file_id: fileRow.id,
        owner_id: owner.id,
        actor_id: editor.id,
        revision: 10,
        source: 'restore',
      }),
    );
    expect(activityInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        owner_id: owner.id,
        actor_id: editor.id,
        file_id: fileRow.id,
        type: 'file_restored',
      }),
    );
    expect(result.data).toMatchObject({ id: fileRow.id, revision: 10 });
  });
});
