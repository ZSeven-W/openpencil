import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns the current selection (immutable string[]).
 * Re-renders only when selection changes.
 */
export function useSelection(): string[] {
  const engine = useDesignEngine();
  return useEngineSubscribe(engine, 'selection:change', (e) => e.getSelection());
}
