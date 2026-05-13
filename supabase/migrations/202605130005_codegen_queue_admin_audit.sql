-- OpenPencil codegen queue permissions, provider config audit, and runtime stats support.
-- Forward-only migration for Supabase Cloud.

create table if not exists public.codegen_queue_members (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  role text not null check (role in ('viewer', 'operator', 'admin')),
  created_by uuid references auth.users(id) on delete set null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (user_id)
);

create table if not exists public.codegen_provider_config_audits (
  id uuid primary key default gen_random_uuid(),
  provider text not null,
  actor_id uuid references auth.users(id) on delete set null,
  actor_email text,
  before_config jsonb not null default '{}'::jsonb,
  after_config jsonb not null default '{}'::jsonb,
  reason text,
  created_at timestamptz not null default now()
);

create index if not exists codegen_queue_members_user_idx
  on public.codegen_queue_members(user_id);

create index if not exists codegen_provider_config_audits_provider_created_idx
  on public.codegen_provider_config_audits(provider, created_at desc);

create index if not exists codegen_provider_config_audits_actor_created_idx
  on public.codegen_provider_config_audits(actor_id, created_at desc);

create index if not exists codegen_jobs_owner_created_provider_idx
  on public.codegen_jobs(owner_id, created_at desc, provider);

alter table public.codegen_queue_members enable row level security;
alter table public.codegen_provider_config_audits enable row level security;

drop policy if exists "codegen queue members are service role only"
  on public.codegen_queue_members;
create policy "codegen queue members are service role only"
on public.codegen_queue_members
for all
using (false)
with check (false);

drop policy if exists "codegen provider config audits are service role only"
  on public.codegen_provider_config_audits;
create policy "codegen provider config audits are service role only"
on public.codegen_provider_config_audits
for all
using (false)
with check (false);
