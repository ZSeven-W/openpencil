-- Make background codegen step rows idempotent per job attempt.
-- Forward-only migration for Supabase Cloud.

alter table public.codegen_job_steps
  add column if not exists attempt integer not null default 1 check (attempt > 0);

with ranked as (
  select
    id,
    job_id,
    agent_role,
    attempt,
    row_number() over (
      partition by job_id, agent_role, attempt
      order by
        case status
          when 'succeeded' then 5
          when 'failed' then 4
          when 'canceled' then 4
          when 'running' then 3
          when 'pending' then 1
          else 0
        end desc,
        updated_at desc,
        created_at desc
    ) as rn
  from public.codegen_job_steps
),
merged as (
  select
    job_id,
    agent_role,
    attempt,
    min(started_at) filter (where started_at is not null) as started_at,
    max(completed_at) filter (where completed_at is not null) as completed_at,
    max(progress) as progress
  from public.codegen_job_steps
  group by job_id, agent_role, attempt
)
update public.codegen_job_steps step
set
  started_at = coalesce(step.started_at, merged.started_at),
  completed_at = coalesce(step.completed_at, merged.completed_at),
  progress = greatest(step.progress, merged.progress),
  updated_at = now()
from ranked
join merged
  on merged.job_id = ranked.job_id
  and merged.agent_role = ranked.agent_role
  and merged.attempt = ranked.attempt
where step.id = ranked.id
  and ranked.rn = 1;

with ranked as (
  select
    id,
    row_number() over (
      partition by job_id, agent_role, attempt
      order by
        case status
          when 'succeeded' then 5
          when 'failed' then 4
          when 'canceled' then 4
          when 'running' then 3
          when 'pending' then 1
          else 0
        end desc,
        updated_at desc,
        created_at desc
    ) as rn
  from public.codegen_job_steps
)
delete from public.codegen_job_steps step
using ranked
where step.id = ranked.id
  and ranked.rn > 1;

create unique index if not exists codegen_job_steps_job_role_attempt_idx
  on public.codegen_job_steps(job_id, agent_role, attempt);
