-- OpenPencil cloud version metadata, labels, conflict overrides, and activity history.
-- Forward-only migration for Supabase Cloud.

alter table public.design_file_versions
  add column if not exists actor_id uuid references auth.users(id) on delete set null,
  add column if not exists size_bytes bigint not null default 0;

create index if not exists design_file_versions_actor_idx
  on public.design_file_versions(actor_id, created_at desc)
  where actor_id is not null;

create table if not exists public.activity_events (
  id uuid primary key default gen_random_uuid(),
  file_id uuid references public.design_files(id) on delete set null,
  generation_id uuid references public.code_generations(id) on delete set null,
  actor_id uuid references auth.users(id) on delete set null,
  owner_id uuid not null references auth.users(id) on delete cascade,
  type text not null check (
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
      'codegen_promoted',
      'codegen_deleted',
      'version_labeled',
      'file_force_saved'
    )
  ),
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists activity_events_owner_created_idx
  on public.activity_events(owner_id, created_at desc);

create index if not exists activity_events_file_created_idx
  on public.activity_events(file_id, created_at desc)
  where file_id is not null;

create index if not exists activity_events_generation_created_idx
  on public.activity_events(generation_id, created_at desc)
  where generation_id is not null;

alter table public.activity_events enable row level security;

drop policy if exists "activity events are owned by user" on public.activity_events;
drop policy if exists "activity events are visible to owners and actors" on public.activity_events;
create policy "activity events are visible to owners and actors"
on public.activity_events
for select
using (owner_id = auth.uid() or actor_id = auth.uid());

drop policy if exists "activity events can be inserted by owners and shared editors" on public.activity_events;
create policy "activity events can be inserted by owners and shared editors"
on public.activity_events
for insert
with check (
  actor_id = auth.uid()
  and (
    owner_id = auth.uid()
    or (
      file_id is not null
      and exists (
        select 1
        from public.design_files df
        join public.design_file_shares dfs on dfs.file_id = df.id
        where df.id = activity_events.file_id
          and df.owner_id = activity_events.owner_id
          and df.deleted_at is null
          and dfs.role = 'editor'
          and dfs.revoked_at is null
          and (
            dfs.shared_with_user_id = auth.uid()
            or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
          )
      )
    )
  )
);

drop policy if exists "file owners can read all file versions" on public.design_file_versions;
create policy "file owners can read all file versions"
on public.design_file_versions
for select
using (
  exists (
    select 1
    from public.design_files df
    where df.id = design_file_versions.file_id
      and df.owner_id = auth.uid()
  )
);

drop policy if exists "editor shared file versions are writable" on public.design_file_versions;
create policy "editor shared file versions are writable"
on public.design_file_versions
for insert
with check (
  actor_id = auth.uid()
  and exists (
    select 1
    from public.design_files df
    join public.design_file_shares dfs on dfs.file_id = df.id
    where df.id = design_file_versions.file_id
      and df.owner_id = design_file_versions.owner_id
      and df.deleted_at is null
      and dfs.role = 'editor'
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
);
