import { describe, expect, it } from 'vitest';
import { isBackgroundJobsMigrationError, toCodegenDbError } from '../cloud-codegen-errors';
import { listCodegenJobs } from '../cloud-codegen-jobs';

describe('cloud codegen migration errors', () => {
  it('treats missing stage 9 reliability columns as a migration-required error', () => {
    const error = {
      code: '42703',
      message: 'column codegen_jobs.last_heartbeat_at does not exist',
    };

    expect(isBackgroundJobsMigrationError(error)).toBe(true);

    const apiError = toCodegenDbError(error, 'Failed to list codegen jobs') as Error & {
      statusCode?: number;
      data?: { error?: { code?: string; details?: { migration?: string } } };
    };
    expect(apiError.statusCode).toBe(503);
    expect(apiError.data?.error?.code).toBe('migration_required');
    expect(apiError.data?.error?.details?.migration).toContain('202605130002');
  });

  it('surfaces list codegen jobs missing columns as migration-required', async () => {
    const query = {
      eq: () => query,
      order: () => query,
      limit: async () => ({
        data: null,
        error: {
          code: '42703',
          message: 'column codegen_jobs.last_heartbeat_at does not exist',
        },
      }),
    };
    const supabase = {
      from: () => ({
        select: () => query,
      }),
    };

    await expect(
      listCodegenJobs({
        supabase: supabase as never,
        userId: 'user-1',
        limit: 50,
      }),
    ).rejects.toMatchObject({
      statusCode: 503,
      data: {
        error: {
          code: 'migration_required',
        },
      },
    });
  });
});
