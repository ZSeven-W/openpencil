-- Backfill pipeline mode from existing job output metadata and provider call timing.

with inferred as (
  select
    job.id,
    coalesce(
      nullif(job.output->>'pipelineMode', ''),
      nullif(job.output->'metadata'->>'pipelineMode', ''),
      case
        when exists (
          select 1
          from jsonb_array_elements(coalesce(job.output->'timing'->'providerCalls', '[]'::jsonb)) call
          where call->>'stage' = 'direct_generation'
        ) or exists (
          select 1
          from jsonb_array_elements(coalesce(job.output->'checkpoints', '[]'::jsonb)) checkpoint
          cross join lateral jsonb_array_elements(
            coalesce(checkpoint->'data'->'timing'->'providerCalls', '[]'::jsonb)
          ) call
          where call->>'stage' = 'direct_generation'
        ) then 'direct_generation'
        when exists (
          select 1
          from jsonb_array_elements(coalesce(job.output->'timing'->'providerCalls', '[]'::jsonb)) call
          where call->>'stage' in ('planning', 'chunk', 'assembly')
        ) or exists (
          select 1
          from jsonb_array_elements(coalesce(job.output->'checkpoints', '[]'::jsonb)) checkpoint
          where checkpoint->>'stage' in ('planning', 'chunk', 'assembly')
        ) or exists (
          select 1
          from jsonb_array_elements(coalesce(job.output->'checkpoints', '[]'::jsonb)) checkpoint
          cross join lateral jsonb_array_elements(
            coalesce(checkpoint->'data'->'timing'->'providerCalls', '[]'::jsonb)
          ) call
          where call->>'stage' in ('planning', 'chunk', 'assembly')
        ) then 'full_pipeline'
        else null
      end
    ) as pipeline_mode
  from public.codegen_jobs job
)
update public.codegen_jobs job
set pipeline_mode = inferred.pipeline_mode
from inferred
where job.id = inferred.id
  and job.pipeline_mode is null
  and inferred.pipeline_mode in ('direct_generation', 'full_pipeline', 'unknown');
