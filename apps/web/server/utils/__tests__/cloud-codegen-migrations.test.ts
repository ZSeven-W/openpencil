import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const ROOT = process.cwd().endsWith('/apps/web') ? join(process.cwd(), '../..') : process.cwd();

describe('cloud codegen migrations', () => {
  it('declares stale queue dead-letter counters in the function that uses them', () => {
    const sql = readFileSync(
      join(ROOT, 'supabase/migrations/202605130002_codegen_queue_reliability.sql'),
      'utf-8',
    );
    const match = /create or replace function public\.requeue_stale_codegen_jobs\(\)[\s\S]*?\$\$;/i.exec(
      sql,
    );

    expect(match?.[0]).toContain('v_dead_lettered integer;');
    expect(match?.[0]).toContain('get diagnostics v_dead_lettered = row_count;');
  });

  it('adds idempotent codegen step attempts before creating the unique key', () => {
    const sql = readFileSync(
      join(ROOT, 'supabase/migrations/202605130008_codegen_job_step_attempts.sql'),
      'utf-8',
    );

    expect(sql).toContain('add column if not exists attempt integer not null default 1');
    expect(sql).toContain('partition by job_id, agent_role, attempt');
    expect(sql).toContain('codegen_job_steps_job_role_attempt_idx');
    expect(sql).toContain('on public.codegen_job_steps(job_id, agent_role, attempt)');
  });
});
