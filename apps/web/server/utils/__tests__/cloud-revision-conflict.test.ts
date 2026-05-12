import { describe, expect, it, vi } from 'vitest';
import {
  createCloudRevisionConflictDetails,
  isOptimisticUpdateMiss,
  throwCloudRevisionConflict,
} from '../cloud-revision-conflict';

function makeSupabaseMock(revision: number | null) {
  const maybeSingle = vi.fn().mockResolvedValue({
    data: revision === null ? null : { revision },
    error: null,
  });
  const eqOwner = vi.fn(() => ({ maybeSingle }));
  const eqId = vi.fn(() => ({ eq: eqOwner }));
  const select = vi.fn(() => ({ eq: eqId }));
  const from = vi.fn(() => ({ select }));
  return { supabase: { from }, from, select, eqId, eqOwner, maybeSingle };
}

describe('cloud revision conflict helpers', () => {
  it('creates stable structured conflict details', () => {
    expect(createCloudRevisionConflictDetails('file-1', 7, 12)).toEqual({
      fileId: 'file-1',
      expectedRevision: 7,
      serverRevision: 12,
    });
  });

  it('recognizes an optimistic update miss from PostgREST single-row errors', () => {
    expect(isOptimisticUpdateMiss(null, { code: 'PGRST116' })).toBe(true);
    expect(isOptimisticUpdateMiss(undefined, null)).toBe(true);
    expect(isOptimisticUpdateMiss({ id: 'file-1' }, { code: 'PGRST116' })).toBe(false);
    expect(isOptimisticUpdateMiss(null, { code: '23505' })).toBe(false);
  });

  it('throws a revision_conflict error with the current server revision', async () => {
    const mock = makeSupabaseMock(12);

    await expect(
      throwCloudRevisionConflict(mock.supabase as never, 'file-1', 'user-1', 7),
    ).rejects.toMatchObject({
      statusCode: 409,
      statusMessage: 'Cloud file has a newer revision',
      data: {
        error: {
          code: 'revision_conflict',
          details: {
            fileId: 'file-1',
            expectedRevision: 7,
            serverRevision: 12,
          },
        },
      },
    });

    expect(mock.from).toHaveBeenCalledWith('design_files');
    expect(mock.select).toHaveBeenCalledWith('revision');
    expect(mock.eqId).toHaveBeenCalledWith('id', 'file-1');
    expect(mock.eqOwner).toHaveBeenCalledWith('owner_id', 'user-1');
  });
});
