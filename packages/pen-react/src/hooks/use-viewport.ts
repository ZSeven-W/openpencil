import { useRef, useCallback } from 'react';
import type { ViewportState } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns viewport state (zoom, panX, panY).
 * Re-renders only on viewport:change events.
 *
 * Caches the snapshot object so that useSyncExternalStore gets a stable
 * reference when values haven't changed — avoids infinite re-render loops.
 */
export function useViewport(): ViewportState {
  const engine = useDesignEngine();
  const cacheRef = useRef<ViewportState | null>(null);
  const getSnapshot = useCallback(
    (e: typeof engine) => {
      const zoom = e.zoom;
      const panX = e.panX;
      const panY = e.panY;
      const prev = cacheRef.current;
      if (prev && prev.zoom === zoom && prev.panX === panX && prev.panY === panY) {
        return prev;
      }
      const next = { zoom, panX, panY };
      cacheRef.current = next;
      return next;
    },
    [engine],
  );
  return useEngineSubscribe(engine, 'viewport:change', getSnapshot);
}
