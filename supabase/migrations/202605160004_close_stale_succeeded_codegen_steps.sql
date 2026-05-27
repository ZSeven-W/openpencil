-- Close stale running/pending step rows for jobs that already succeeded.
-- Older workers could leave an early step, usually planner, in running state
-- after later stages and the job itself had completed successfully.

with role_order(agent_role, sort_order) as (
  values
    ('planner', 0),
    ('page_codegen', 1),
    ('reviewer_repair', 2),
    ('quality_check', 3),
    ('repair', 4),
    ('final_validation', 5),
    ('bundler_persist', 6)
)
update public.codegen_job_steps step
set
  status = 'succeeded',
  progress = greatest(coalesce(step.progress, 0), 100),
  completed_at = coalesce(step.completed_at, job.completed_at, job.updated_at, now()),
  updated_at = now()
from public.codegen_jobs job
cross join role_order step_role
where step.job_id = job.id
  and step_role.agent_role = step.agent_role
  and job.status = 'succeeded'
  and step.status in ('pending', 'running')
  and exists (
    select 1
    from public.codegen_job_steps later
    join role_order later_role on later_role.agent_role = later.agent_role
    where later.job_id = step.job_id
      and later.attempt = step.attempt
      and later.status = 'succeeded'
      and later_role.sort_order > step_role.sort_order
  );
