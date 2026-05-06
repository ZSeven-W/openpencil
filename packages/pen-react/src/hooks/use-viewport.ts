import { useRef, useCallback } from 'react';
import type { ViewportState } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns
 * 视口状态（缩放、panX、panY）。 Re-仅在 viewport:change
 *
 * 事件上渲染。 Caches 快照对象，以便 useSyncExternalStore
 * 在值未更改时获得稳定的引
 用 - 避免无限重新渲染循环。
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
