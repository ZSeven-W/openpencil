import { useMemo } from 'react';
import type { PenNode } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useSelection } from './use-selection.js';
import { useDocument } from './use-document.js';

/**
 * Returns
 * 第一个选定节点的完整数据，或者为 null。 Derived 来自 useSelection +
 *
 * useDocument — 在任一更改上重新渲染。 `doc` 包含在 deps 中，因此即使选择数组保持不变，当节点属性发生更改时，备忘录
 * 也会重新计算。
 */
export function useActiveNode(): PenNode | null {
  const engine = useDesignEngine();
  const selection = useSelection();
  const doc = useDocument();

  return useMemo(() => {
    if (selection.length === 0) return null;
    return engine.getNodeById(selection[0]) ?? null;
  }, [engine, selection, doc]);
}
