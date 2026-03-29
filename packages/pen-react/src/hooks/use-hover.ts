import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns the currently hovered node ID, or null.
 * Re-renders only on node:hover events.
 */
export function useHover(): string | null {
  const engine = useDesignEngine();
  return useEngineSubscribe(engine, 'node:hover', (e) => {
    // Engine stores hover state internally; getSnapshot retrieves it.
    // This relies on the engine exposing a getter for hover state.
    // For now, the engine returns the last emitted hover ID.
    return (e as any).getHoveredId?.() ?? null;
  });
}
