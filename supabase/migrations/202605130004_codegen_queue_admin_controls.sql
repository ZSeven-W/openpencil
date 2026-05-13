-- OpenPencil codegen queue admin controls and replay audit.
-- Forward-only migration for Supabase Cloud.

create table if not exists public.codegen_provider_configs (
  provider text primary key,
  enabled boolean not null default true,
  max_per_minute integer not null default 30 check (max_per_minute > 0),
  circuit_threshold integer not null default 3 check (circuit_threshold > 0),
  circuit_open_ms integer not null default 60000 check (circuit_open_ms >= 1000),
  updated_by uuid references auth.users(id) on delete set null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

insert into public.codegen_provider_configs(
  provider,
  enabled,
  max_per_minute,
  circuit_threshold,
  circuit_open_ms
)
values
  ('openai', true, 30, 3, 60000),
  ('gemini', true, 30, 3, 60000)
on conflict (provider) do nothing;

create index if not exists codegen_provider_configs_updated_idx
  on public.codegen_provider_configs(updated_at desc);

create index if not exists codegen_jobs_owner_provider_failure_dead_idx
  on public.codegen_jobs(owner_id, provider, failure_type, dead_lettered_at desc)
  where dead_lettered_at is not null;

alter table public.codegen_provider_configs enable row level security;

drop policy if exists "codegen provider configs are service role only"
  on public.codegen_provider_configs;
create policy "codegen provider configs are service role only"
on public.codegen_provider_configs
for all
using (false)
with check (false);

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
      'codegen_job_replayed',
      'codegen_promoted',
      'codegen_deleted',
      'version_labeled',
      'file_force_saved'
    )
  );
