import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns the currently hovered node ID, or null.
 * Re-renders only on node:hover events.
 */
export function useHover(): string | null {
  const engine = useDesignEngine();
  return useEngineSubscribe(engine, 'node:hover', (e) => e.getHoveredId());
}
