import { useCallback, useRef, useSyncExternalStore } from 'react';
import type { OpViewer } from '@zseven-w/op-web-sdk';
import { useViewer } from './use-viewer.js';

type ViewerEvent = 'load' | 'viewportchange';

// Subscribe to `event` and return a referentially stable snapshot between events.
//
// CRITICAL — render-loop prevention: the core's getters (e.g. `viewport`) return
// a fresh object on every call. If we passed them directly to useSyncExternalStore,
// React would see a new reference on every render, trigger a re-render, call the
// getter again, get yet another new reference, and loop infinitely.
//
// Solution: cache the last snapshot in a ref. The subscribe callback clears the
// cache when the event fires so the *next* getSnapshot call fetches a fresh value.
// Between events the same cached object is returned, satisfying reference stability.
function useViewerSnapshot<T>(event: ViewerEvent, read: (v: OpViewer) => T): T {
  const viewer = useViewer();
  // Null signals "cache is dirty, read fresh on next getSnapshot call".
  const cache = useRef<{ value: T } | null>(null);

  const subscribe = useCallback(
    (cb: () => void) =>
      viewer.on(event, () => {
        // Invalidate the cache so getSnapshot returns the new value after this event.
        cache.current = null;
        cb();
      }),
    [viewer, event],
  );

  const getSnapshot = useCallback(() => {
    if (cache.current === null) {
      cache.current = { value: read(viewer) };
    }
    return cache.current.value;
  }, [viewer, read]);

  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

// Returns the current PenDocument; re-renders when 'load' fires.
export function useDocument() {
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return useViewerSnapshot('load', useCallback((v: OpViewer) => v.document, []));
}

// Returns the current viewport {panX, panY, zoom}; re-renders when 'viewportchange' fires.
export function useViewport() {
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return useViewerSnapshot('viewportchange', useCallback((v: OpViewer) => v.viewport, []));
}

// Returns the active page index; re-renders when 'load' fires.
export function useActivePage(): number {
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return useViewerSnapshot('load', useCallback((v: OpViewer) => v.activePage, []));
}
