import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { H3Event } from 'h3';

const h3Mocks = vi.hoisted(() => ({
  body: {} as unknown,
  query: {} as Record<string, string>,
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

vi.mock('h3', async () => {
  const actual = await vi.importActual<typeof import('h3')>('h3');
  return {
    ...actual,
    defineEventHandler: (handler: unknown) => handler,
    getQuery: vi.fn(() => h3Mocks.query),
    readBody: vi.fn(async () => h3Mocks.body),
    setResponseStatus: vi.fn((_event: unknown, status: number) => {
      h3Mocks.status = status;
    }),
  };
});

vi.mock('../../../utils/cloud-supabase', () => cloudSupabaseMocks);
vi.mock('../../../utils/cloud-codegen-assets', () => ({
  addSignedUrlsToCodegenManifest: vi.fn(async (_supabase: unknown, manifest: unknown) => manifest),
}));

const user = { id: 'user-1', email: 'alice@example.com' };
const event = {} as H3Event;
const generationRow = {
  id: '66666666-6666-4666-8666-666666666666',
  file_id: '33333333-3333-4333-8333-333333333333',
  page_id: 'page-1',
  framework: 'uniapp',
  target_kind: 'page',
  node_ids: [],
  target_hash: 'hash-1',
  document_revision: 4,
  status: 'done',
  final_code: '<template><view>Shared</view></template>',
  entry_file: 'pages/index/index.vue',
  degraded: false,
  assets_manifest: [],
  model: 'model-a',
  provider: 'builtin',
  error: null,
  created_at: '2026-05-12T08:00:00.000Z',
  completed_at: '2026-05-12T08:00:02.000Z',
  promoted_at: null,
};

function createListQuery(data: unknown, error: unknown = null) {
  const query = {
    select: vi.fn(() => query),
    eq: vi.fn(() => query),
    order: vi.fn(() => query),
    limit: vi.fn(async () => ({ data, error })),
  };
  return query;
}

beforeEach(() => {
  vi.clearAllMocks();
  h3Mocks.body = {};
  h3Mocks.query = {};
  h3Mocks.status = undefined;
  cloudSupabaseMocks.getCloudSupabase.mockReset();
});

describe('cloud code generation routes', () => {
  it('lists history by owner, cloud file, and target so web and desktop share the same records', async () => {
    const listQuery = createListQuery([generationRow]);
    const from = vi.fn((table: string) => {
      if (table === 'code_generations') return { select: listQuery.select };
      throw new Error(`Unexpected table ${table}`);
    });
    cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase: { from }, user });
    h3Mocks.query = {
      fileId: generationRow.file_id,
      pageId: 'page-1',
      framework: 'uniapp',
      targetKind: 'page',
      targetHash: 'hash-1',
      limit: '10',
    };

    const handler = (await import('../code-generations/index.get')).default;
    const result = await handler(event);

    expect(from).toHaveBeenCalledWith('code_generations');
    expect(listQuery.eq).toHaveBeenCalledWith('owner_id', user.id);
    expect(listQuery.eq).toHaveBeenCalledWith('file_id', generationRow.file_id);
    expect(listQuery.eq).toHaveBeenCalledWith('page_id', 'page-1');
    expect(listQuery.eq).toHaveBeenCalledWith('framework', 'uniapp');
    expect(listQuery.eq).toHaveBeenCalledWith('target_kind', 'page');
    expect(listQuery.eq).toHaveBeenCalledWith('target_hash', 'hash-1');
    expect(listQuery.order).toHaveBeenCalledWith('created_at', { ascending: false });
    expect(listQuery.limit).toHaveBeenCalledWith(10);
    expect(result.data).toEqual([
      expect.objectContaining({
        id: generationRow.id,
        fileId: generationRow.file_id,
        framework: 'uniapp',
        finalCode: '<template><view>Shared</view></template>',
      }),
    ]);
  });
});
