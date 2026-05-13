-- OpenPencil background codegen jobs and task notifications.
-- Forward-only migration for Supabase Cloud.

create table if not exists public.codegen_jobs (
  id uuid primary key default gen_random_uuid(),
  file_id uuid not null references public.design_files(id) on delete cascade,
  owner_id uuid not null references auth.users(id) on delete cascade,
  generation_id uuid references public.code_generations(id) on delete set null,
  status text not null default 'pending' check (
    status in ('pending', 'running', 'succeeded', 'failed', 'canceled')
  ),
  framework text not null,
  page_id text not null,
  target_kind text not null check (target_kind in ('page', 'selection')),
  node_ids text[] not null default '{}'::text[],
  target_hash text not null,
  document_revision integer not null check (document_revision > 0),
  provider text,
  model text,
  priority integer not null default 0,
  progress integer not null default 0 check (progress >= 0 and progress <= 100),
  attempts integer not null default 0 check (attempts >= 0),
  max_attempts integer not null default 2 check (max_attempts > 0),
  locked_by text,
  locked_until timestamptz,
  input_snapshot jsonb not null default '{}'::jsonb,
  output jsonb not null default '{}'::jsonb,
  error text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  started_at timestamptz,
  completed_at timestamptz,
  canceled_at timestamptz
);

create index if not exists codegen_jobs_owner_created_idx
  on public.codegen_jobs(owner_id, created_at desc);

create index if not exists codegen_jobs_owner_status_created_idx
  on public.codegen_jobs(owner_id, status, created_at desc);

create index if not exists codegen_jobs_file_created_idx
  on public.codegen_jobs(file_id, created_at desc);

create index if not exists codegen_jobs_pending_priority_idx
  on public.codegen_jobs(status, priority desc, created_at asc)
  where status = 'pending';

create table if not exists public.codegen_job_steps (
  id uuid primary key default gen_random_uuid(),
  job_id uuid not null references public.codegen_jobs(id) on delete cascade,
  agent_role text not null check (
    agent_role in ('planner', 'page_codegen', 'reviewer_repair', 'bundler_persist')
  ),
  status text not null default 'pending' check (
    status in ('pending', 'running', 'succeeded', 'failed', 'canceled')
  ),
  progress integer not null default 0 check (progress >= 0 and progress <= 100),
  input jsonb not null default '{}'::jsonb,
  output jsonb not null default '{}'::jsonb,
  error text,
  started_at timestamptz,
  completed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists codegen_job_steps_job_idx
  on public.codegen_job_steps(job_id, created_at asc);

create table if not exists public.codegen_job_events (
  id uuid primary key default gen_random_uuid(),
  job_id uuid not null references public.codegen_jobs(id) on delete cascade,
  type text not null,
  message text,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists codegen_job_events_job_created_idx
  on public.codegen_job_events(job_id, created_at asc);

create table if not exists public.task_notifications (
  id uuid primary key default gen_random_uuid(),
  owner_id uuid not null references auth.users(id) on delete cascade,
  job_id uuid references public.codegen_jobs(id) on delete cascade,
  file_id uuid references public.design_files(id) on delete set null,
  generation_id uuid references public.code_generations(id) on delete set null,
  kind text not null check (
    kind in ('codegen_job_succeeded', 'codegen_job_failed', 'codegen_job_canceled')
  ),
  title text not null,
  message text,
  read_at timestamptz,
  created_at timestamptz not null default now()
);

create index if not exists task_notifications_owner_created_idx
  on public.task_notifications(owner_id, created_at desc);

create index if not exists task_notifications_owner_unread_idx
  on public.task_notifications(owner_id, created_at desc)
  where read_at is null;

alter table public.codegen_jobs enable row level security;
alter table public.codegen_job_steps enable row level security;
alter table public.codegen_job_events enable row level security;
alter table public.task_notifications enable row level security;

drop policy if exists "codegen jobs are owned by user" on public.codegen_jobs;
create policy "codegen jobs are owned by user"
on public.codegen_jobs
for all
using (owner_id = auth.uid())
with check (owner_id = auth.uid());

drop policy if exists "codegen job steps follow owned jobs" on public.codegen_job_steps;
create policy "codegen job steps follow owned jobs"
on public.codegen_job_steps
for all
using (
  exists (
    select 1 from public.codegen_jobs cj
    where cj.id = job_id and cj.owner_id = auth.uid()
  )
)
with check (
  exists (
    select 1 from public.codegen_jobs cj
    where cj.id = job_id and cj.owner_id = auth.uid()
  )
);

drop policy if exists "codegen job events follow owned jobs" on public.codegen_job_events;
create policy "codegen job events follow owned jobs"
on public.codegen_job_events
for all
using (
  exists (
    select 1 from public.codegen_jobs cj
    where cj.id = job_id and cj.owner_id = auth.uid()
  )
)
with check (
  exists (
    select 1 from public.codegen_jobs cj
    where cj.id = job_id and cj.owner_id = auth.uid()
  )
);

drop policy if exists "task notifications are owned by user" on public.task_notifications;
create policy "task notifications are owned by user"
on public.task_notifications
for all
using (owner_id = auth.uid())
with check (owner_id = auth.uid());

alter table public.activity_events
  drop constraint if exists activity_events_type_check;

alter table public.activity_events
  add constraint activity_events_type_check check (
    type in (
      'file_created',
      'file_saved',
      'file_restored',
      'file_renamed',
      'file_moved',
      'file_shared',
      'file_share_revoked',
      'file_deleted',
      'file_restored_from_trash',
      'file_permanently_deleted',
      'codegen_created',
      'codegen_job_created',
      'codegen_job_succeeded',
      'codegen_job_failed',
      'codegen_job_canceled',
      'codegen_promoted',
      'codegen_deleted',
      'version_labeled',
      'file_force_saved'
    )
  );

