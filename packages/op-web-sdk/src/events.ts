// Typed event emitter for OpViewer lifecycle and viewport change events.

/** Events emitted by OpViewer. */
export type ViewerEvent = 'load' | 'viewportchange';

/** Minimal typed pub/sub emitter used internally by OpViewer. */
export class Emitter {
  private map = new Map<ViewerEvent, Set<() => void>>();

  /** Subscribe to an event. Returns an unsubscribe function. */
  on(e: ViewerEvent, cb: () => void): () => void {
    let set = this.map.get(e);
    if (!set) { set = new Set(); this.map.set(e, set); }
    set.add(cb);
    return () => this.off(e, cb);
  }

  /** Unsubscribe a specific callback from an event. */
  off(e: ViewerEvent, cb: () => void): void { this.map.get(e)?.delete(cb); }

  /** Fire all listeners registered for the given event. */
  emit(e: ViewerEvent): void { this.map.get(e)?.forEach((cb) => cb()); }

  /** Remove all listeners for all events. */
  clear(): void { this.map.clear(); }
}
