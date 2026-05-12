-- OpenPencil cloud projects, folders, and full file management metadata.
-- Forward-only migration for Supabase Cloud.

create table if not exists public.projects (
  id uuid primary key default gen_random_uuid(),
  owner_id uuid not null references auth.users(id) on delete cascade,
  name text not null,
  description text,
  icon text,
  color text,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.folders (
  id uuid primary key default gen_random_uuid(),
  project_id uuid not null references public.projects(id) on delete cascade,
  owner_id uuid not null references auth.users(id) on delete cascade,
  parent_id uuid references public.folders(id) on delete cascade,
  name text not null,
  sort_order integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

alter table public.design_files
  add column if not exists project_id uuid references public.projects(id) on delete set null,
  add column if not exists folder_id uuid references public.folders(id) on delete set null,
  add column if not exists metadata jsonb not null default '{}'::jsonb,
  add column if not exists last_opened_at timestamptz,
  add column if not exists starred boolean not null default false;

create index if not exists projects_owner_active_idx
  on public.projects(owner_id, updated_at desc)
  where deleted_at is null;

create index if not exists folders_project_parent_active_idx
  on public.folders(project_id, parent_id, sort_order, name)
  where deleted_at is null;

create index if not exists folders_owner_active_idx
  on public.folders(owner_id, updated_at desc)
  where deleted_at is null;

create index if not exists design_files_project_folder_active_idx
  on public.design_files(owner_id, project_id, folder_id, updated_at desc)
  where deleted_at is null;

create index if not exists design_files_owner_starred_idx
  on public.design_files(owner_id, updated_at desc)
  where deleted_at is null and starred = true;

create index if not exists design_files_owner_recent_idx
  on public.design_files(owner_id, last_opened_at desc)
  where deleted_at is null and last_opened_at is not null;

create index if not exists design_files_owner_deleted_idx
  on public.design_files(owner_id, deleted_at desc)
  where deleted_at is not null;

create index if not exists design_files_owner_name_idx
  on public.design_files(owner_id, lower(name))
  where deleted_at is null;

drop trigger if exists projects_set_updated_at on public.projects;
create trigger projects_set_updated_at
before update on public.projects
for each row execute function public.set_updated_at();

drop trigger if exists folders_set_updated_at on public.folders;
create trigger folders_set_updated_at
before update on public.folders
for each row execute function public.set_updated_at();

alter table public.projects enable row level security;
alter table public.folders enable row level security;

drop policy if exists "projects are owned by user" on public.projects;
create policy "projects are owned by user"
on public.projects
for all
using (owner_id = auth.uid())
with check (owner_id = auth.uid());

drop policy if exists "folders are owned by user" on public.folders;
create policy "folders are owned by user"
on public.folders
for all
using (owner_id = auth.uid())
with check (owner_id = auth.uid());

create or replace function public.ensure_default_project(target_user_id uuid)
returns uuid
language plpgsql
security definer
set search_path = public
as $$
declare
  default_project_id uuid;
begin
  select id into default_project_id
  from public.projects
  where owner_id = target_user_id
    and deleted_at is null
  order by created_at asc
  limit 1;

  if default_project_id is null then
    insert into public.projects (owner_id, name)
    values (target_user_id, 'My Project')
    returning id into default_project_id;
  end if;

  update public.design_files
  set project_id = default_project_id
  where owner_id = target_user_id
    and public.design_files.project_id is null;

  return default_project_id;
end;
$$;

create or replace function public.handle_new_user()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  insert into public.profiles (id, email, display_name)
  values (new.id, new.email, coalesce(new.raw_user_meta_data->>'display_name', split_part(new.email, '@', 1)))
  on conflict (id) do update
    set email = excluded.email,
        display_name = coalesce(public.profiles.display_name, excluded.display_name),
        updated_at = now();

  perform public.ensure_default_project(new.id);
  return new;
end;
$$;
