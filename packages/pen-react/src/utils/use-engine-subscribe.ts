import { useSyncExternalStore, useCallback, useRef } from 'react';
import type { DesignEngine, DesignEngineEvents } from '@zseven-w/pen-engine';

/**
 * Generic hook to subscribe to engine events via useSyncExternalStore.
 *
 * getSnapshot MUST return a stable reference when state hasn't changed.
 * The engine guarantees immutable refs — see Immutability contract in pen-engine.
 *
 * The engine's `on()` method returns an unsubscribe function, which is
 * exactly what useSyncExternalStore's `subscribe` callback expects.
 *
 * We keep the snap function reference stable across renders by storing the
 * latest getSnapshot in a ref. This prevents infinite loops when callers
 * pass inline arrow functions as getSnapshot.
 */
export function useEngineSubscribe<K extends keyof DesignEngineEvents, T>(
  engine: DesignEngine,
  event: K,
  getSnapshot: (engine: DesignEngine) => T,
): T {
  const snapshotRef = useRef(getSnapshot);
  snapshotRef.current = getSnapshot;

  const subscribe = useCallback((cb: () => void) => engine.on(event, cb as any), [engine, event]);
  const snap = useCallback(() => snapshotRef.current(engine), [engine]);
  return useSyncExternalStore(subscribe, snap);
}
