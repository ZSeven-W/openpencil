-- OpenPencil codegen patch jobs.
-- Forward-only migration for Supabase Cloud.

alter table public.codegen_jobs
  add column if not exists job_kind text not null default 'full_generation';

alter table public.codegen_jobs
  drop constraint if exists codegen_jobs_job_kind_check;

alter table public.codegen_jobs
  add constraint codegen_jobs_job_kind_check check (
    job_kind in ('full_generation', 'patch_generation')
  );

create index if not exists codegen_jobs_owner_kind_created_idx
  on public.codegen_jobs(owner_id, job_kind, created_at desc);
