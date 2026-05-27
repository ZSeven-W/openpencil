-- OpenPencil codegen job list performance indexes.
-- Optimizes task center and worker management list queries after moving large
-- task payload columns out of list selects.

create index if not exists codegen_jobs_owner_created_cover_idx
  on public.codegen_jobs(owner_id, created_at desc)
  include (
    id,
    file_id,
    generation_id,
    job_kind,
    status,
    framework,
    provider,
    model,
    priority,
    progress,
    attempts,
    max_attempts,
    dead_lettered_at,
    failure_type
  );

create index if not exists codegen_jobs_owner_status_created_cover_idx
  on public.codegen_jobs(owner_id, status, created_at desc)
  include (
    id,
    file_id,
    generation_id,
    job_kind,
    framework,
    provider,
    model,
    priority,
    progress,
    attempts,
    max_attempts,
    dead_lettered_at,
    failure_type
  );

create index if not exists codegen_jobs_owner_file_created_idx
  on public.codegen_jobs(owner_id, file_id, created_at desc);

create index if not exists codegen_jobs_owner_dead_lettered_cover_idx
  on public.codegen_jobs(owner_id, dead_lettered_at desc)
  where dead_lettered_at is not null;

create index if not exists codegen_jobs_owner_failed_dead_lettered_idx
  on public.codegen_jobs(owner_id, status, dead_lettered_at desc)
  where status = 'failed' and dead_lettered_at is not null;

create index if not exists codegen_jobs_owner_provider_failure_dead_lettered_idx
  on public.codegen_jobs(owner_id, provider, failure_type, dead_lettered_at desc)
  where status = 'failed' and dead_lettered_at is not null;
