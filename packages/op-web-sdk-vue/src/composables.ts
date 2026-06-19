import { onScopeDispose, shallowRef, triggerRef, watch, type ShallowRef } from 'vue';
import type { OpViewer, PenDocument } from '@zseven-w/op-web-sdk';
import { useViewer } from './use-viewer.js';

type ViewerEvent = 'load' | 'viewportchange';

/**
 * Watch the viewer ref and keep an output ref in sync with a viewer property.
 * When the viewer becomes available (or changes), subscribe to `event` and
 * read the initial value immediately. The previous subscription is cleaned up
 * via watch's onCleanup callback.
 */
function useViewerRef<T>(
  event: ViewerEvent,
  read: (v: OpViewer) => T,
  fallback: T,
): Readonly<ShallowRef<T>> {
  const viewerRef = useViewer();
  const out = shallowRef<T>(viewerRef.value != null ? read(viewerRef.value) : fallback);

  // Track the latest unsubscribe function for onScopeDispose safety.
  let latestOff: (() => void) | null = null;

  watch(
    viewerRef,
    (v, _old, onCleanup) => {
      if (!v) return;
      // Read initial value as soon as the viewer is available.
      out.value = read(v);
      triggerRef(out);
      // Subscribe to future events from this viewer.
      const off = v.on(event, () => {
        out.value = read(v);
        triggerRef(out);
      });
      latestOff = off;
      onCleanup(off);
    },
    { immediate: true },
  );

  // Safety: also unsubscribe when the composable scope is disposed (e.g. component unmounted
  // before the watcher's own cleanup runs).
  onScopeDispose(() => {
    latestOff?.();
    latestOff = null;
  });

  return out;
}

export function useDocument(): Readonly<ShallowRef<PenDocument | null>> {
  return useViewerRef('load', (v) => v.document, null);
}

export function useViewport() {
  return useViewerRef('viewportchange', (v) => v.viewport, null);
}

export function useActivePage(): Readonly<ShallowRef<number>> {
  return useViewerRef('load', (v) => v.activePage, 0);
}
