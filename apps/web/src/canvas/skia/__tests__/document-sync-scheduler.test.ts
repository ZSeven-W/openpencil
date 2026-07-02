import { describe, expect, it } from 'vitest';

import { createDocumentSyncScheduler } from '../document-sync-scheduler';

describe('document sync scheduler', () => {
  it('coalesces repeated schedule calls into one sync per frame', () => {
    const rafQueue: Array<(time: number) => void> = [];
    const engine = {
      dragSyncSuppressed: false,
      syncFromDocumentCalls: 0,
      syncFromDocument() {
        this.syncFromDocumentCalls += 1;
      },
    };

    const scheduler = createDocumentSyncScheduler(
      () => engine,
      (cb) => {
        rafQueue.push(cb);
        return rafQueue.length;
      },
      () => {},
    );

    scheduler.schedule();
    scheduler.schedule();
    scheduler.schedule();

    expect(rafQueue).toHaveLength(1);
    expect(engine.syncFromDocumentCalls).toBe(0);

    const cb = rafQueue.shift();
    cb?.(0);

    expect(engine.syncFromDocumentCalls).toBe(1);
  });

  it('retries on the next frame while drag sync is suppressed', () => {
    const rafQueue: Array<(time: number) => void> = [];
    const engine = {
      dragSyncSuppressed: true,
      syncFromDocumentCalls: 0,
      syncFromDocument() {
        this.syncFromDocumentCalls += 1;
      },
    };

    const scheduler = createDocumentSyncScheduler(
      () => engine,
      (cb) => {
        rafQueue.push(cb);
        return rafQueue.length;
      },
      () => {},
    );

    scheduler.schedule();
    expect(rafQueue).toHaveLength(1);

    rafQueue.shift()?.(0);
    expect(engine.syncFromDocumentCalls).toBe(0);
    expect(rafQueue).toHaveLength(1);

    engine.dragSyncSuppressed = false;
    rafQueue.shift()?.(16);
    expect(engine.syncFromDocumentCalls).toBe(1);
  });
});
