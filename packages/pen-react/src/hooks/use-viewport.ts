import type { ViewportState } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns viewport state (zoom, panX, panY).
 * Re-renders only on viewport:change events.
 *
 * Note: viewport getters return primitive values. The engine constructs a
 * new ViewportState object only on mutation, ensuring stable refs for
 * useSyncExternalStore.
 */
export function useViewport(): ViewportState {
  const engine = useDesignEngine();
  return useEngineSubscribe(engine, 'viewport:change', (e) => ({
    zoom: e.zoom,
    panX: e.panX,
    panY: e.panY,
  }));
}
