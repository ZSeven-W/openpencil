import type { PenDocument } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns 当前
 * PenDocument （不可变引用）。 Re-仅在文档发生变化时渲染（通过结构共享生成新引用）
 。
 */
export function useDocument(): PenDocument {
  const engine = useDesignEngine();
  return useEngineSubscribe(engine, 'document:change', (e) => e.getDocument());
}
