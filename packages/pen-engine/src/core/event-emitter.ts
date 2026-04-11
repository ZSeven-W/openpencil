export class TypedEventEmitter<Events extends { [K in keyof Events]: (...args: any[]) => void }> {
  private listeners = new Map<keyof Events, Set<(...args: any[]) => void>>();

  on<K extends keyof Events>(event: K, cb: Events[K]): () => void {
    let set = this.listeners.get(event);
    if (!set) {
      set = new Set();
      this.listeners.set(event, set);
    }
    set.add(cb as (...args: any[]) => void);
    return () => this.off(event, cb);
  }

  off<K extends keyof Events>(event: K, cb: Events[K]): void {
    this.listeners.get(event)?.delete(cb as (...args: any[]) => void);
  }

  emit<K extends keyof Events>(event: K, ...args: Parameters<Events[K]>): void {
    const set = this.listeners.get(event);
    if (!set) return;
    for (const cb of set) {
      cb(...args);
    }
  }

  dispose(): void {
    this.listeners.clear();
  }
}
