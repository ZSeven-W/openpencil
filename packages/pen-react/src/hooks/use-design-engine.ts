import { useContext } from 'react';
import type { DesignEngine } from '@zseven-w/pen-engine';
import { DesignEngineContext } from '../context.js';

/**
 * Get 来自最近的
 * <DesignProvider> 的 DesignEngine 实例。 Throws 如果在提供商外部使用。
 */
export function useDesignEngine(): DesignEngine {
  const engine = useContext(DesignEngineContext);
  if (!engine) {
    throw new Error(
      'useDesignEngine must be used within a <DesignProvider>. ' +
        'Wrap your component tree with <DesignProvider>.',
    );
  }
  return engine;
}
