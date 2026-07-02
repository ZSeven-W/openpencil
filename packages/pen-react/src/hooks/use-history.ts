import { useCallback, useRef } from 'react';
import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

interface HistoryState {
  canUndo: boolean;
  canRedo: boolean;
  undo: () => void;
  redo: () => void;
}

/**
 * Returns undo/redo availability and action functions.
 * Re-renders only on history:change events.
 *
 * The snapshot object is cached by reference so that useSyncExternalStore
 * does not trigger an infinite re-render loop (new object === new snapshot).
 */
export function useHistory(): HistoryState {
  const engine = useDesignEngine();
  const cacheRef = useRef<{ canUndo: boolean; canRedo: boolean } | null>(null);
  const getSnapshot = useCallback(
    (e: typeof engine) => {
      const canUndo = !!e.canUndo;
      const canRedo = !!e.canRedo;
      if (
        cacheRef.current &&
        cacheRef.current.canUndo === canUndo &&
        cacheRef.current.canRedo === canRedo
      ) {
        return cacheRef.current;
      }
      cacheRef.current = { canUndo, canRedo };
      return cacheRef.current;
    },
    [engine],
  );
  const state = useEngineSubscribe(engine, 'history:change', getSnapshot);
  const undo = useCallback(() => engine.undo(), [engine]);
  const redo = useCallback(() => engine.redo(), [engine]);
  return { ...state, undo, redo };
}
