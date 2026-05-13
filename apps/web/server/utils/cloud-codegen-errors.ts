import { createError } from 'h3';
import { toApiError } from './cloud-supabase';

export const BACKGROUND_JOBS_MIGRATION =
  [
    'supabase/migrations/202605130001_background_codegen_jobs.sql',
    'supabase/migrations/202605130002_codegen_queue_reliability.sql',
    'supabase/migrations/202605130003_codegen_worker_management.sql',
    'supabase/migrations/202605130004_codegen_queue_admin_controls.sql',
    'supabase/migrations/202605130005_codegen_queue_admin_audit.sql',
    'supabase/migrations/202605130006_codegen_patch_jobs.sql',
    'supabase/migrations/202605130007_codegen_generation_metadata.sql',
    'supabase/migrations/202605130008_codegen_job_step_attempts.sql',
  ].join(', ');

export interface SupabaseErrorLike {
  code?: string;
  message?: string;
  details?: string;
  hint?: string;
}

export function isBackgroundJobsMigrationError(error: SupabaseErrorLike): boolean {
  const text = [error.code, error.message, error.details, error.hint]
    .filter(Boolean)
    .join(' ')
    .toLowerCase();
  const mentionsBackgroundTable =
    text.includes('codegen_jobs') ||
    text.includes('codegen_job_steps') ||
    text.includes('codegen_job_events') ||
    text.includes('task_notifications') ||
    text.includes('codegen_worker_heartbeats') ||
    text.includes('codegen_queue_members') ||
    text.includes('codegen_provider_configs') ||
    text.includes('codegen_provider_config_audits') ||
    text.includes('codegen_provider_health') ||
    text.includes('codegen_provider_rate_windows') ||
    text.includes('last_heartbeat_at') ||
    text.includes('next_run_at') ||
    text.includes('dead_lettered_at') ||
    text.includes('last_error') ||
    text.includes('failure_type') ||
    text.includes('reserve_codegen_provider_capacity') ||
    text.includes('claim_next_codegen_job') ||
    text.includes('heartbeat_codegen_job') ||
    text.includes('requeue_stale_codegen_jobs');
  return (
    mentionsBackgroundTable &&
    (text.includes('schema cache') ||
      text.includes('does not exist') ||
      text.includes('could not find') ||
      text.includes('column') ||
      text.includes('function') ||
      text.includes('relation') ||
      text.includes('undefined_table') ||
      text.includes('undefined_column') ||
      text.includes('undefined_function') ||
      text.includes('42p01') ||
      text.includes('42703') ||
      text.includes('42883') ||
      text.includes('pgrst204') ||
      text.includes('pgrst205'))
  );
}

export function toCodegenDbError(error: SupabaseErrorLike, fallback: string) {
  if (isBackgroundJobsMigrationError(error)) {
    return toApiError(
      503,
      'migration_required',
      'Background task tables are not initialized. Run the background codegen jobs migration.',
      {
        migration: BACKGROUND_JOBS_MIGRATION,
        originalCode: error.code,
        originalMessage: error.message,
      },
    );
  }
  return createError({ statusCode: 500, statusMessage: error.message ?? fallback });
}
