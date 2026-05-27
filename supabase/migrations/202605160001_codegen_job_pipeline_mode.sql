-- Store lightweight codegen pipeline mode for task list views without loading heavy output checkpoints.

alter table public.codegen_jobs
  add column if not exists pipeline_mode text;

do $$
begin
  alter table public.codegen_jobs
    add constraint codegen_jobs_pipeline_mode_check
    check (
      pipeline_mode is null
      or pipeline_mode in ('direct_generation', 'full_pipeline', 'unknown')
    );
exception
  when duplicate_object then null;
end $$;
