import { useMemo } from 'react';
import type { PenNode } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useSelection } from './use-selection.js';
import { useDocument } from './use-document.js';

/**
 * Returns the first selected node's full data, or null.
 * Derived from useSelection + useDocument — re-renders on either change.
 */
export function useActiveNode(): PenNode | null {
  const engine = useDesignEngine();
  const selection = useSelection();
  // Subscribe to document so we get updated node data when props change
  useDocument();

  return useMemo(() => {
    if (selection.length === 0) return null;
    return engine.getNodeById(selection[0]) ?? null;
  }, [engine, selection]);
}
