-- Allow production codegen stage roles in background job steps.
-- The initial background queue only allowed the first four agent roles, but
-- quality gates and stage resume now persist dedicated quality/repair steps.

alter table public.codegen_job_steps
  drop constraint if exists codegen_job_steps_agent_role_check;

alter table public.codegen_job_steps
  add constraint codegen_job_steps_agent_role_check
  check (
    agent_role in (
      'planner',
      'page_codegen',
      'reviewer_repair',
      'quality_check',
      'repair',
      'final_validation',
      'bundler_persist'
    )
  );
