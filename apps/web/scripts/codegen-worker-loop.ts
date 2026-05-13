import type { SupabaseClient } from '@supabase/supabase-js';
import type { CloudCodegenJob } from '../src/types/cloud';
import type { CodegenWorkerRuntimeOptions } from '../server/utils/cloud-codegen-job-queue';

export type CodegenWorkerRunOnce = (
  supabase: SupabaseClient,
  options: CodegenWorkerRuntimeOptions,
) => Promise<CloudCodegenJob | null>;

interface CodegenWorkerLoopOptions {
  supabase: SupabaseClient;
  workerId: string;
  lockMs: number;
  retryDelayMs: number;
  pollIntervalMs: number;
  runOnce: CodegenWorkerRunOnce;
  logger?: Pick<Console, 'error'>;
}

export function createCodegenWorkerLoop(options: CodegenWorkerLoopOptions) {
  let stopping = false;
  let running = false;
  let timer: ReturnType<typeof setInterval> | undefined;
  const logger = options.logger ?? console;

  const tick = async () => {
    if (running || stopping) return null;
    running = true;
    try {
      return await options.runOnce(options.supabase, {
        workerId: options.workerId,
        lockMs: options.lockMs,
        retryDelayMs: options.retryDelayMs,
      });
    } catch (err) {
      logger.error('[codegen-worker] tick failed', err);
      return null;
    } finally {
      running = false;
    }
  };

  const start = () => {
    if (timer) return;
    timer = setInterval(() => {
      void tick();
    }, options.pollIntervalMs);
    void tick();
  };

  const stop = () => {
    stopping = true;
    if (timer) clearInterval(timer);
    timer = undefined;
  };

  return {
    tick,
    start,
    stop,
    isRunning: () => running,
    isStopping: () => stopping,
  };
}
