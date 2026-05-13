import { loadEnvFiles } from './env';
import { createCodegenWorkerLoop } from './codegen-worker-loop';
import { getCloudServiceSupabase } from '../server/utils/cloud-supabase';
import { runCodegenWorkerOnce } from '../server/utils/cloud-codegen-jobs';

const loadedEnv = loadEnvFiles();
const POLL_INTERVAL_MS = Number(process.env.OPENPENCIL_CODEGEN_WORKER_INTERVAL_MS ?? 10_000);
const LOCK_MS = Number(process.env.OPENPENCIL_CODEGEN_WORKER_LOCK_MS ?? 5 * 60_000);
const RETRY_DELAY_MS = Number(process.env.OPENPENCIL_CODEGEN_WORKER_RETRY_DELAY_MS ?? 30_000);
const WORKER_ID =
  process.env.OPENPENCIL_CODEGEN_WORKER_ID ?? `standalone-${process.pid}-${Date.now()}`;

function missingWorkerEnvKeys() {
  const missing: string[] = [];
  if (!process.env.SUPABASE_URL && !process.env.VITE_SUPABASE_URL) {
    missing.push('SUPABASE_URL or VITE_SUPABASE_URL');
  }
  if (!process.env.SUPABASE_SERVICE_ROLE_KEY) {
    missing.push('SUPABASE_SERVICE_ROLE_KEY');
  }
  return missing;
}

const supabase = getCloudServiceSupabase();
if (!supabase) {
  const loaded = loadedEnv.loaded.map((file) => file.relativePath).join(', ') || 'none';
  const missing = missingWorkerEnvKeys().join(', ') || 'cloud worker credentials';
  throw new Error(
    [
      'Cannot start OpenPencil codegen worker.',
      `Missing ${missing}.`,
      'Put server-only values in .env.local or apps/web/.env.local, or export them before running.',
      'Use `bun --bun run dev:cloud` to start the web app and worker together.',
      `Loaded env files: ${loaded}.`,
    ].join(' '),
  );
}
const worker = createCodegenWorkerLoop({
  supabase,
  workerId: WORKER_ID,
  lockMs: LOCK_MS,
  retryDelayMs: RETRY_DELAY_MS,
  pollIntervalMs: POLL_INTERVAL_MS,
  runOnce: runCodegenWorkerOnce,
});

process.on('SIGINT', () => {
  worker.stop();
});
process.on('SIGTERM', () => {
  worker.stop();
});

worker.start();
