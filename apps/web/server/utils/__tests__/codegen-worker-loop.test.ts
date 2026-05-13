import { afterEach, describe, expect, it, vi } from 'vitest';
import { createCodegenWorkerLoop } from '../../../scripts/codegen-worker-loop';

afterEach(() => {
  vi.useRealTimers();
});

describe('standalone codegen worker loop', () => {
  it('ticks immediately on start so pending jobs are consumed without manual polling', async () => {
    vi.useFakeTimers();
    const supabase = { from: vi.fn() } as never;
    const runOnce = vi.fn(async () => null);
    const worker = createCodegenWorkerLoop({
      supabase,
      workerId: 'standalone-test',
      lockMs: 120_000,
      retryDelayMs: 30_000,
      pollIntervalMs: 10_000,
      runOnce,
      logger: { error: vi.fn() },
    });

    worker.start();
    await Promise.resolve();

    expect(runOnce).toHaveBeenCalledTimes(1);
    expect(runOnce).toHaveBeenLastCalledWith(supabase, {
      workerId: 'standalone-test',
      lockMs: 120_000,
      retryDelayMs: 30_000,
    });

    await vi.advanceTimersByTimeAsync(10_000);

    expect(runOnce).toHaveBeenCalledTimes(2);
    worker.stop();
  });

  it('does not start overlapping queue ticks', async () => {
    let releaseRun: (() => void) | undefined;
    const runOnce = vi.fn(
      () =>
        new Promise<null>((resolve) => {
          releaseRun = () => resolve(null);
        }),
    );
    const worker = createCodegenWorkerLoop({
      supabase: {} as never,
      workerId: 'standalone-test',
      lockMs: 120_000,
      retryDelayMs: 30_000,
      pollIntervalMs: 10_000,
      runOnce,
      logger: { error: vi.fn() },
    });

    const firstTick = worker.tick();
    await worker.tick();

    expect(runOnce).toHaveBeenCalledTimes(1);
    releaseRun?.();
    await firstTick;
  });
});
