interface SyncableEngine {
  dragSyncSuppressed: boolean;
  syncFromDocument: () => void;
}

export function createDocumentSyncScheduler(
  getEngine: () => SyncableEngine | null,
  requestFrame: (cb: (time: number) => void) => number = (cb) => requestAnimationFrame(cb),
  cancelFrame: (id: number) => void = (id) => cancelAnimationFrame(id),
) {
  let frameId = 0;
  let pending = false;
  let disposed = false;

  const flush = () => {
    frameId = 0;
    pending = false;
    if (disposed) return;

    const engine = getEngine();
    if (!engine) return;

    if (engine.dragSyncSuppressed) {
      schedule();
      return;
    }

    engine.syncFromDocument();
  };

  const schedule = () => {
    if (disposed || pending) return;
    pending = true;
    frameId = requestFrame(flush);
  };

  const dispose = () => {
    disposed = true;
    pending = false;
    if (frameId) {
      cancelFrame(frameId);
      frameId = 0;
    }
  };

  return { schedule, dispose };
}
