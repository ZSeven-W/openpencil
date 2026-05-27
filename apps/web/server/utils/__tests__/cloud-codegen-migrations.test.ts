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

  it('allows quality and resume stage agent roles in codegen job steps', () => {
    const sql = readFileSync(
      join(ROOT, 'supabase/migrations/202605160003_codegen_job_step_agent_roles.sql'),
      'utf-8',
    );

    expect(sql).toContain('drop constraint if exists codegen_job_steps_agent_role_check');
    expect(sql).toContain('quality_check');
    expect(sql).toContain('repair');
    expect(sql).toContain('final_validation');
  });

  it('closes stale running codegen steps for succeeded jobs', () => {
    const sql = readFileSync(
      join(ROOT, 'supabase/migrations/202605160004_close_stale_succeeded_codegen_steps.sql'),
      'utf-8',
    );

    expect(sql).toContain("job.status = 'succeeded'");
    expect(sql).toContain("step.status in ('pending', 'running')");
    expect(sql).toContain("later.status = 'succeeded'");
    expect(sql).toContain("status = 'succeeded'");
  });

  it('adds list-oriented indexes for codegen job summary queries', () => {
    const sql = readFileSync(
      join(ROOT, 'supabase/migrations/202605130009_codegen_job_list_indexes.sql'),
      'utf-8',
    );

    expect(sql).toContain('codegen_jobs_owner_created_cover_idx');
    expect(sql).toContain('codegen_jobs_owner_status_created_cover_idx');
    expect(sql).toContain('codegen_jobs_owner_file_created_idx');
    expect(sql).toContain('codegen_jobs_owner_failed_dead_lettered_idx');
    expect(sql).toContain('where status = ');
    expect(sql).not.toContain('input_snapshot');
    expect(sql).not.toContain('output');
  });

  it('adds lightweight codegen job display names for list views', () => {
    const sql = readFileSync(
      join(ROOT, 'supabase/migrations/202605130011_codegen_job_display_names.sql'),
      'utf-8',
    );

    expect(sql).toContain('add column if not exists file_name text');
    expect(sql).toContain('add column if not exists page_name text');
    expect(sql).toContain("input_snapshot->>'fileName'");
    expect(sql).toContain("input_snapshot->>'pageName'");
  });
});
