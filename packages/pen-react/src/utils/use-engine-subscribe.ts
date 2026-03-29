import { useSyncExternalStore, useCallback } from 'react';
import type { DesignEngine, DesignEngineEvents } from '@zseven-w/pen-engine';

/**
 * Generic hook to subscribe to engine events via useSyncExternalStore.
 *
 * getSnapshot MUST return a stable reference when state hasn't changed.
 * The engine guarantees immutable refs — see Immutability contract in pen-engine.
 *
 * The engine's `on()` method returns an unsubscribe function, which is
 * exactly what useSyncExternalStore's `subscribe` callback expects.
 */
export function useEngineSubscribe<K extends keyof DesignEngineEvents, T>(
  engine: DesignEngine,
  event: K,
  getSnapshot: (engine: DesignEngine) => T,
): T {
  const subscribe = useCallback(
    (cb: () => void) => engine.on(event, cb as any),
    [engine, event],
  );
  const snap = useCallback(() => getSnapshot(engine), [engine, getSnapshot]);
  return useSyncExternalStore(subscribe, snap);
}
