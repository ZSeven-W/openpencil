import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns
 * 当前悬停的节点 ID，或 null。 Re-仅在节点：悬停事件上渲染。
 */
export function useHover(): string | null {
  const engine = useDesignEngine();
  return useEngineSubscribe(engine, 'node:hover', (e) => e.getHoveredId());
}
