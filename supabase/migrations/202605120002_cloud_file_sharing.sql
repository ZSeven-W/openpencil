-- OpenPencil cloud file sharing.
-- Forward-only migration for Supabase Cloud.

create table if not exists public.design_file_shares (
  id uuid primary key default gen_random_uuid(),
  file_id uuid not null references public.design_files(id) on delete cascade,
  owner_id uuid not null references auth.users(id) on delete cascade,
  shared_with_user_id uuid references auth.users(id) on delete cascade,
  shared_with_email text,
  role text not null default 'viewer' check (role in ('viewer', 'editor')),
  revoked_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint design_file_shares_recipient_check check (
    shared_with_user_id is not null or shared_with_email is not null
  )
);

create index if not exists design_file_shares_owner_file_active_idx
  on public.design_file_shares(owner_id, file_id, created_at desc)
  where revoked_at is null;

create index if not exists design_file_shares_recipient_user_active_idx
  on public.design_file_shares(shared_with_user_id, created_at desc)
  where revoked_at is null and shared_with_user_id is not null;

create index if not exists design_file_shares_recipient_email_active_idx
  on public.design_file_shares(shared_with_email, created_at desc)
  where revoked_at is null and shared_with_email is not null;

create unique index if not exists design_file_shares_file_user_active_idx
  on public.design_file_shares(file_id, shared_with_user_id)
  where revoked_at is null and shared_with_user_id is not null;

create unique index if not exists design_file_shares_file_email_active_idx
  on public.design_file_shares(file_id, lower(shared_with_email))
  where revoked_at is null and shared_with_email is not null;

drop trigger if exists design_file_shares_set_updated_at on public.design_file_shares;
create trigger design_file_shares_set_updated_at
before update on public.design_file_shares
for each row execute function public.set_updated_at();

alter table public.design_file_shares enable row level security;

drop policy if exists "file shares are visible to owners and recipients" on public.design_file_shares;
create policy "file shares are visible to owners and recipients"
on public.design_file_shares
for select
using (
  owner_id = auth.uid()
  or (
    revoked_at is null
    and (
      shared_with_user_id = auth.uid()
      or lower(coalesce(shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
    )
  )
);

drop policy if exists "file owners can create shares" on public.design_file_shares;
create policy "file owners can create shares"
on public.design_file_shares
for insert
with check (
  owner_id = auth.uid()
  and exists (
    select 1
    from public.design_files df
    where df.id = file_id
      and df.owner_id = auth.uid()
      and df.deleted_at is null
  )
);

drop policy if exists "file owners can update shares" on public.design_file_shares;
create policy "file owners can update shares"
on public.design_file_shares
for update
using (owner_id = auth.uid())
with check (owner_id = auth.uid());

drop policy if exists "file owners can delete shares" on public.design_file_shares;
create policy "file owners can delete shares"
on public.design_file_shares
for delete
using (owner_id = auth.uid());

drop policy if exists "shared design files are readable" on public.design_files;
create policy "shared design files are readable"
on public.design_files
for select
using (
  deleted_at is null
  and exists (
    select 1
    from public.design_file_shares dfs
    where dfs.file_id = design_files.id
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
);

drop policy if exists "editor shared design files are editable" on public.design_files;
create policy "editor shared design files are editable"
on public.design_files
for update
using (
  deleted_at is null
  and exists (
    select 1
    from public.design_file_shares dfs
    where dfs.file_id = design_files.id
      and dfs.role = 'editor'
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
)
with check (
  deleted_at is null
  and exists (
    select 1
    from public.design_file_shares dfs
    where dfs.file_id = design_files.id
      and dfs.role = 'editor'
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
);

drop policy if exists "editor shared file versions are readable" on public.design_file_versions;
create policy "editor shared file versions are readable"
on public.design_file_versions
for select
using (
  exists (
    select 1
    from public.design_file_shares dfs
    where dfs.file_id = design_file_versions.file_id
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
);

drop policy if exists "editor shared file versions are writable" on public.design_file_versions;
create policy "editor shared file versions are writable"
on public.design_file_versions
for insert
with check (
  owner_id = auth.uid()
  and exists (
    select 1
    from public.design_file_shares dfs
    where dfs.file_id = design_file_versions.file_id
      and dfs.role = 'editor'
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
);
