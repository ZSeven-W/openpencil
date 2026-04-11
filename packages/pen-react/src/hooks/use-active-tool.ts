import type { ToolType } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns the currently active tool type.
 * Re-renders only on tool:change events.
 */
export function useActiveTool(): [ToolType, (tool: ToolType) => void] {
  const engine = useDesignEngine();
  const tool = useEngineSubscribe(engine, 'tool:change', (e) => e.getActiveTool());
  return [tool, (t: ToolType) => engine.setActiveTool(t)];
}
