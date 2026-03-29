import type { PenDocument } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';

/**
 * Returns the current PenDocument (immutable ref).
 * Re-renders only when the document mutates (new ref via structural sharing).
 */
export function useDocument(): PenDocument {
  const engine = useDesignEngine();
  return useEngineSubscribe(engine, 'document:change', (e) => e.getDocument());
}
