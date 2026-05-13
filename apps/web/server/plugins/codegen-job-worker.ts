import { getCloudServiceSupabase } from '../utils/cloud-supabase';
import { runCodegenWorkerOnce } from '../utils/cloud-codegen-jobs';
import { resolveEmbeddedCodegenWorkerMode } from '../utils/cloud-codegen-job-queue';

const POLL_INTERVAL_MS = 10_000;
const WORKER_ID = `nitro-${process.pid}`;
const LOCK_MS = Number(process.env.OPENPENCIL_CODEGEN_WORKER_LOCK_MS ?? 5 * 60_000);
const RETRY_DELAY_MS = Number(process.env.OPENPENCIL_CODEGEN_WORKER_RETRY_DELAY_MS ?? 30_000);

export default () => {
  if (!resolveEmbeddedCodegenWorkerMode().enabled) return;
  const supabase = getCloudServiceSupabase();
  if (!supabase) return;

  let running = false;
  const tick = () => {
    if (running) return;
    running = true;
    runCodegenWorkerOnce(supabase, {
      workerId: process.env.OPENPENCIL_CODEGEN_WORKER_ID ?? WORKER_ID,
      lockMs: LOCK_MS,
      retryDelayMs: RETRY_DELAY_MS,
    })
      .catch((err) => {
        console.error('[codegen-job-worker] poll failed', err);
      })
      .finally(() => {
        running = false;
      });
  };

  const timer = setInterval(tick, POLL_INTERVAL_MS);
  tick();

  const cleanup = () => clearInterval(timer);
  process.on('SIGINT', cleanup);
  process.on('SIGTERM', cleanup);
};
