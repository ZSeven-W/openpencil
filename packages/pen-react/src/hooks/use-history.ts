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
 * Returns
 * undo/redo 可用性和操作功能。 Re-仅渲染历史记录：更改事件。 The
 *
 * 快照对象通过引用进行缓存，以便 useSyncExternalStore
 * 不会触发无限重新渲染循环
 （新对象 === 新快照）。
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
