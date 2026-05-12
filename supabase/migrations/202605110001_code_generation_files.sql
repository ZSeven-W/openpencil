-- OpenPencil code generation multi-file output.
-- Forward-only migration for Supabase Cloud.

alter table public.code_generations
  add column if not exists entry_file text;

alter table public.code_generations
  add column if not exists promoted_at timestamptz;

create table if not exists public.code_generation_files (
  id uuid primary key default gen_random_uuid(),
  generation_id uuid not null references public.code_generations(id) on delete cascade,
  path text not null check (
    length(path) > 0
    and path !~ '^/'
    and path !~ '(^|/)\.\.(/|$)'
  ),
  language text not null default 'text',
  role text not null default 'other' check (
    role in ('entry', 'page', 'component', 'style', 'config', 'asset', 'other')
  ),
  content text not null,
  size_bytes bigint not null default 0,
  order_index integer not null default 0,
  created_at timestamptz not null default now(),
  unique (generation_id, path)
);

create index if not exists code_generation_files_generation_idx
  on public.code_generation_files(generation_id, order_index);

alter table public.code_generation_files enable row level security;

drop policy if exists "code generation files follow owned generations" on public.code_generation_files;
create policy "code generation files follow owned generations"
on public.code_generation_files
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
