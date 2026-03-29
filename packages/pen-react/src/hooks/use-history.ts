import { useCallback } from 'react';
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
 */
export function useHistory(): HistoryState {
  const engine = useDesignEngine();
  const state = useEngineSubscribe(engine, 'history:change', (e) => ({
    canUndo: e.canUndo,
    canRedo: e.canRedo,
  }));
  const undo = useCallback(() => engine.undo(), [engine]);
  const redo = useCallback(() => engine.redo(), [engine]);
  return { ...state, undo, redo };
}
