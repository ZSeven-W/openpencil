import { useContext } from 'react';
import type { DesignEngine } from '@zseven-w/pen-engine';
import { DesignEngineContext } from '../context.js';

/**
 * Get the DesignEngine instance from the nearest <DesignProvider>.
 * Throws if used outside a provider.
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
