-- Store lightweight task display names for list views without loading input snapshots.

alter table public.codegen_jobs
  add column if not exists file_name text,
  add column if not exists page_name text;

update public.codegen_jobs
set
  file_name = coalesce(file_name, nullif(input_snapshot->>'fileName', '')),
  page_name = coalesce(page_name, nullif(input_snapshot->>'pageName', ''))
where file_name is null or page_name is null;
