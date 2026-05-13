-- OpenPencil codegen queue reliability fields and atomic queue RPCs.
-- Forward-only migration for Supabase Cloud.

alter table public.codegen_jobs
  add column if not exists last_heartbeat_at timestamptz,
  add column if not exists next_run_at timestamptz not null default now(),
  add column if not exists dead_lettered_at timestamptz,
  add column if not exists last_error text,
  add column if not exists failure_type text;

alter table public.codegen_jobs
  drop constraint if exists codegen_jobs_failure_type_check;

alter table public.codegen_jobs
  add constraint codegen_jobs_failure_type_check check (
    failure_type is null or failure_type in (
      'rate_limit',
      'timeout',
      'provider_error',
      'canceled',
      'execution_error'
    )
  );

create index if not exists codegen_jobs_runnable_priority_idx
  on public.codegen_jobs(priority desc, created_at asc)
  where status = 'pending' and dead_lettered_at is null;

create index if not exists codegen_jobs_stale_running_idx
  on public.codegen_jobs(locked_until asc)
  where status = 'running' and locked_until is not null;

create index if not exists codegen_jobs_dead_lettered_idx
  on public.codegen_jobs(owner_id, dead_lettered_at desc)
  where dead_lettered_at is not null;

create table if not exists public.codegen_worker_heartbeats (
  worker_id text primary key,
  pid integer,
  hostname text,
  metadata jsonb not null default '{}'::jsonb,
  started_at timestamptz not null default now(),
  last_heartbeat_at timestamptz not null default now()
);

alter table public.codegen_worker_heartbeats enable row level security;

drop policy if exists "codegen worker heartbeats are service role only" on public.codegen_worker_heartbeats;
create policy "codegen worker heartbeats are service role only"
on public.codegen_worker_heartbeats
for all
using (false)
with check (false);

create or replace function public.claim_next_codegen_job(
  p_worker_id text,
  p_lock_seconds integer default 300
)
returns setof public.codegen_jobs
language plpgsql
security definer
set search_path = public
as $$
begin
  perform public.requeue_stale_codegen_jobs();

  insert into public.codegen_worker_heartbeats(worker_id, pid, metadata, last_heartbeat_at)
  values (
    p_worker_id,
    null,
    jsonb_build_object('source', 'claim_next_codegen_job'),
    now()
  )
  on conflict (worker_id)
  do update set last_heartbeat_at = excluded.last_heartbeat_at;

  return query
  with candidate as (
    select id
    from public.codegen_jobs
    where status = 'pending'
      and attempts < max_attempts
      and dead_lettered_at is null
      and coalesce(next_run_at, created_at) <= now()
    order by priority desc, created_at asc
    limit 1
    for update skip locked
  )
  update public.codegen_jobs job
  set
    status = 'running',
    attempts = job.attempts + 1,
    locked_by = p_worker_id,
    locked_until = now() + make_interval(secs => greatest(p_lock_seconds, 1)),
    last_heartbeat_at = now(),
    started_at = coalesce(job.started_at, now()),
    updated_at = now()
  from candidate
  where job.id = candidate.id
  returning job.*;
end;
$$;

create or replace function public.heartbeat_codegen_job(
  p_job_id uuid,
  p_worker_id text,
  p_lock_seconds integer default 300
)
returns boolean
language plpgsql
security definer
set search_path = public
as $$
declare
  v_updated integer;
begin
  insert into public.codegen_worker_heartbeats(worker_id, pid, metadata, last_heartbeat_at)
  values (
    p_worker_id,
    null,
    jsonb_build_object('source', 'heartbeat_codegen_job'),
    now()
  )
  on conflict (worker_id)
  do update set last_heartbeat_at = excluded.last_heartbeat_at;

  update public.codegen_jobs
  set
    locked_until = now() + make_interval(secs => greatest(p_lock_seconds, 1)),
    last_heartbeat_at = now(),
    updated_at = now()
  where id = p_job_id
    and status = 'running'
    and locked_by = p_worker_id;

  get diagnostics v_updated = row_count;
  return v_updated > 0;
end;
$$;

create or replace function public.requeue_stale_codegen_jobs()
returns integer
language plpgsql
security definer
set search_path = public
as $$
declare
  v_updated integer;
  v_dead_lettered integer;
begin
  update public.codegen_jobs
  set
    status = 'pending',
    locked_by = null,
    locked_until = null,
    last_heartbeat_at = null,
    next_run_at = now(),
    last_error = coalesce(last_error, 'Worker lock expired before completion.'),
    failure_type = coalesce(failure_type, 'timeout'),
    updated_at = now()
  where status = 'running'
    and locked_until is not null
    and locked_until < now()
    and attempts < max_attempts;

  get diagnostics v_updated = row_count;

  update public.codegen_jobs
  set
    status = 'failed',
    progress = 100,
    locked_by = null,
    locked_until = null,
    last_heartbeat_at = null,
    dead_lettered_at = now(),
    completed_at = now(),
    last_error = coalesce(last_error, 'Worker lock expired after the final attempt.'),
    failure_type = coalesce(failure_type, 'timeout'),
    error = coalesce(error, last_error, 'Worker lock expired after the final attempt.'),
    updated_at = now()
  where status = 'running'
    and locked_until is not null
    and locked_until < now()
    and attempts >= max_attempts;

  get diagnostics v_dead_lettered = row_count;
  v_updated = v_updated + v_dead_lettered;
  return v_updated;
end;
$$;

revoke execute on function public.claim_next_codegen_job(text, integer) from public, anon, authenticated;
revoke execute on function public.heartbeat_codegen_job(uuid, text, integer) from public, anon, authenticated;
revoke execute on function public.requeue_stale_codegen_jobs() from public, anon, authenticated;

grant execute on function public.claim_next_codegen_job(text, integer) to service_role;
grant execute on function public.heartbeat_codegen_job(uuid, text, integer) to service_role;
grant execute on function public.requeue_stale_codegen_jobs() to service_role;
