import type { ToolType } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns
 * 当前活动的工具类型。 Re-仅在工具：更改事件上呈现。
 */
export function useActiveTool(): [ToolType, (tool: ToolType) => void] {
  const engine = useDesignEngine();
  const tool = useEngineSubscribe(engine, 'tool:change', (e) => e.getActiveTool());
  return [tool, (t: ToolType) => engine.setActiveTool(t)];
}
