-- OpenPencil cloud file library and code generation history.
-- Forward-only migration for Supabase Cloud.

create extension if not exists pgcrypto;

create table if not exists public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  email text,
  display_name text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.design_files (
  id uuid primary key default gen_random_uuid(),
  owner_id uuid not null references auth.users(id) on delete cascade,
  name text not null,
  document jsonb not null,
  thumbnail_path text,
  revision bigint not null default 1,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.design_file_versions (
  id uuid primary key default gen_random_uuid(),
  file_id uuid not null references public.design_files(id) on delete cascade,
  owner_id uuid not null references auth.users(id) on delete cascade,
  revision bigint not null,
  document jsonb not null,
  source text not null check (source in ('manual_save', 'autosave', 'import', 'code_generation', 'restore')),
  label text,
  created_at timestamptz not null default now()
);

create table if not exists public.code_generations (
  id uuid primary key default gen_random_uuid(),
  file_id uuid not null references public.design_files(id) on delete cascade,
  owner_id uuid not null references auth.users(id) on delete cascade,
  page_id text not null,
  framework text not null,
  target_kind text not null check (target_kind in ('page', 'selection')),
  node_ids text[] not null default '{}',
  target_hash text not null,
  document_revision bigint not null,
  status text not null check (status in ('done', 'degraded', 'failed')),
  final_code text,
  degraded boolean not null default false,
  assets_manifest jsonb not null default '[]'::jsonb,
  model text,
  provider text,
  error text,
  created_at timestamptz not null default now(),
  completed_at timestamptz
);

create table if not exists public.code_generation_chunks (
  id uuid primary key default gen_random_uuid(),
  generation_id uuid not null references public.code_generations(id) on delete cascade,
  chunk_id text not null,
  name text not null,
  status text not null check (status in ('pending', 'running', 'done', 'degraded', 'failed', 'skipped')),
  code text,
  contract jsonb,
  error text,
  order_index integer not null default 0,
  created_at timestamptz not null default now()
);

create table if not exists public.file_assets (
  id uuid primary key default gen_random_uuid(),
  file_id uuid not null references public.design_files(id) on delete cascade,
  owner_id uuid not null references auth.users(id) on delete cascade,
  kind text not null check (kind in ('thumbnail', 'codegen_asset', 'export')),
  bucket text not null default 'openpencil-assets',
  path text not null,
  mime_type text,
  size_bytes bigint,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists design_files_owner_active_idx
  on public.design_files(owner_id, updated_at desc)
  where deleted_at is null;

create unique index if not exists design_file_versions_file_revision_idx
  on public.design_file_versions(file_id, revision);

create index if not exists code_generations_lookup_idx
  on public.code_generations(file_id, page_id, framework, target_kind, target_hash, created_at desc);

create index if not exists code_generation_chunks_generation_idx
  on public.code_generation_chunks(generation_id, order_index);

create index if not exists file_assets_file_idx
  on public.file_assets(file_id, created_at desc);

create or replace function public.set_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

drop trigger if exists profiles_set_updated_at on public.profiles;
create trigger profiles_set_updated_at
before update on public.profiles
for each row execute function public.set_updated_at();

drop trigger if exists design_files_set_updated_at on public.design_files;
create trigger design_files_set_updated_at
before update on public.design_files
for each row execute function public.set_updated_at();

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
  return new;
end;
$$;

drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created
after insert on auth.users
for each row execute function public.handle_new_user();

alter table public.profiles enable row level security;
alter table public.design_files enable row level security;
alter table public.design_file_versions enable row level security;
alter table public.code_generations enable row level security;
alter table public.code_generation_chunks enable row level security;
alter table public.file_assets enable row level security;

drop policy if exists "profiles are owned by user" on public.profiles;
create policy "profiles are owned by user"
on public.profiles
for all
using (id = auth.uid())
with check (id = auth.uid());

drop policy if exists "design files are owned by user" on public.design_files;
create policy "design files are owned by user"
on public.design_files
for all
using (owner_id = auth.uid())
with check (owner_id = auth.uid());

drop policy if exists "file versions are owned by user" on public.design_file_versions;
create policy "file versions are owned by user"
on public.design_file_versions
for all
using (owner_id = auth.uid())
with check (owner_id = auth.uid());

drop policy if exists "code generations are owned by user" on public.code_generations;
create policy "code generations are owned by user"
on public.code_generations
for all
using (owner_id = auth.uid())
with check (owner_id = auth.uid());

drop policy if exists "code generation chunks follow owned generations" on public.code_generation_chunks;
create policy "code generation chunks follow owned generations"
on public.code_generation_chunks
for all
using (
  exists (
    select 1
    from public.code_generations cg
    where cg.id = generation_id
      and cg.owner_id = auth.uid()
  )
)
with check (
  exists (
    select 1
    from public.code_generations cg
    where cg.id = generation_id
      and cg.owner_id = auth.uid()
  )
);

drop policy if exists "file assets are owned by user" on public.file_assets;
create policy "file assets are owned by user"
on public.file_assets
for all
using (owner_id = auth.uid())
with check (owner_id = auth.uid());

insert into storage.buckets (id, name, public)
values ('openpencil-assets', 'openpencil-assets', false)
on conflict (id) do update set public = false;

drop policy if exists "openpencil asset objects are user scoped" on storage.objects;
create policy "openpencil asset objects are user scoped"
on storage.objects
for all
using (
  bucket_id = 'openpencil-assets'
  and (storage.foldername(name))[1] = auth.uid()::text
)
with check (
  bucket_id = 'openpencil-assets'
  and (storage.foldername(name))[1] = auth.uid()::text
);

