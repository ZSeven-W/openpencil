import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns
 * 当前选择（不可变字符串[]）。 Re-仅在选择更改时渲染。
 */
export function useSelection(): string[] {
  const engine = useDesignEngine();
  return useEngineSubscribe(engine, 'selection:change', (e) => e.getSelection());
}
