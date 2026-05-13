-- OpenPencil code generation metadata.
-- Forward-only migration for Supabase Cloud.

alter table public.code_generations
  add column if not exists metadata jsonb not null default '{}'::jsonb;
