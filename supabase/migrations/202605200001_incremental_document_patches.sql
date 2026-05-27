-- Incremental cloud document saves: patch log plus periodic checkpoints.
-- Forward-only migration for Supabase Cloud.

alter table public.design_files
  add column if not exists checkpoint_revision bigint,
  add column if not exists checkpoint_size_bytes bigint not null default 0;

update public.design_files
set checkpoint_revision = revision
where checkpoint_revision is null;

alter table public.design_files
  alter column checkpoint_revision set not null;

create table if not exists public.design_file_changes (
  id uuid primary key default gen_random_uuid(),
  file_id uuid not null references public.design_files(id) on delete cascade,
  owner_id uuid not null references auth.users(id) on delete cascade,
  actor_id uuid references auth.users(id) on delete set null,
  base_revision bigint not null,
  revision bigint not null,
  client_mutation_id text not null,
  patches jsonb not null,
  source text not null default 'autosave' check (source in ('manual_save', 'autosave', 'code_generation')),
  label text,
  size_bytes bigint not null default 0,
  created_at timestamptz not null default now(),
  constraint design_file_changes_revision_progress_check check (revision = base_revision + 1)
);

create unique index if not exists design_file_changes_file_client_mutation_idx
  on public.design_file_changes(file_id, client_mutation_id);

create unique index if not exists design_file_changes_file_revision_idx
  on public.design_file_changes(file_id, revision);

create index if not exists design_file_changes_file_created_idx
  on public.design_file_changes(file_id, created_at desc);

create index if not exists design_file_changes_actor_created_idx
  on public.design_file_changes(actor_id, created_at desc)
  where actor_id is not null;

alter table public.design_file_changes enable row level security;

drop policy if exists "file owners can read document changes" on public.design_file_changes;
create policy "file owners can read document changes"
on public.design_file_changes
for select
using (
  exists (
    select 1
    from public.design_files df
    where df.id = design_file_changes.file_id
      and df.owner_id = auth.uid()
  )
);

drop policy if exists "shared recipients can read document changes" on public.design_file_changes;
create policy "shared recipients can read document changes"
on public.design_file_changes
for select
using (
  exists (
    select 1
    from public.design_files df
    join public.design_file_shares dfs on dfs.file_id = df.id
    where df.id = design_file_changes.file_id
      and df.owner_id = design_file_changes.owner_id
      and df.deleted_at is null
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
);

drop policy if exists "file owners can insert document changes" on public.design_file_changes;
create policy "file owners can insert document changes"
on public.design_file_changes
for insert
with check (
  owner_id = auth.uid()
  and (actor_id is null or actor_id = auth.uid())
  and exists (
    select 1
    from public.design_files df
    where df.id = design_file_changes.file_id
      and df.owner_id = auth.uid()
      and df.deleted_at is null
  )
);

drop policy if exists "shared editors can insert document changes" on public.design_file_changes;
create policy "shared editors can insert document changes"
on public.design_file_changes
for insert
with check (
  actor_id = auth.uid()
  and exists (
    select 1
    from public.design_files df
    join public.design_file_shares dfs on dfs.file_id = df.id
    where df.id = design_file_changes.file_id
      and df.owner_id = design_file_changes.owner_id
      and df.deleted_at is null
      and dfs.role = 'editor'
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
);

grant select, insert on public.design_file_changes to authenticated;
