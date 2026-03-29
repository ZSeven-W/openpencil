import { useCallback } from 'react';
import { SquaresUnite, SquaresSubtract, SquaresIntersect } from 'lucide-react';
import { useDesignEngine } from '../hooks/use-design-engine.js';
import { useSelection } from '../hooks/use-selection.js';
import { canBooleanOp, executeBooleanOp, type BooleanOpType } from '@zseven-w/pen-core';
import type { PenNode } from '@zseven-w/pen-types';

const OPS: Array<{
  type: BooleanOpType;
  icon: typeof SquaresUnite;
  label: string;
  shortcut: string;
}> = [
  { type: 'union', icon: SquaresUnite, label: 'Boolean Union', shortcut: '⌘⌥U' },
  { type: 'subtract', icon: SquaresSubtract, label: 'Boolean Subtract', shortcut: '⌘⌥S' },
  { type: 'intersect', icon: SquaresIntersect, label: 'Boolean Intersect', shortcut: '⌘⌥I' },
];

export function BooleanToolbar() {
  const engine = useDesignEngine();
  const selectedIds = useSelection();

  const nodes = selectedIds
    .map((id) => engine.getNodeById(id))
    .filter((n): n is PenNode => n != null);

  const show = canBooleanOp(nodes);

  const handleOp = useCallback(
    (opType: BooleanOpType) => {
      const currentNodes = selectedIds
        .map((id) => engine.getNodeById(id))
        .filter((n): n is PenNode => n != null);

      if (!canBooleanOp(currentNodes)) return;
      const result = executeBooleanOp(currentNodes, opType);
      if (!result) return;

      for (const id of selectedIds) {
        engine.removeNode(id);
      }
      engine.addNode(null, result);
      engine.select([result.id]);
    },
    [engine, selectedIds],
  );

  if (!show) return null;

  return (
    <div className="absolute top-2 left-14 z-10 bg-card border border-border rounded-xl flex items-center py-1 px-1 gap-0.5 shadow-lg">
      {OPS.map((op) => {
        const Icon = op.icon;
        return (
          <button
            key={op.type}
            type="button"
            onClick={() => handleOp(op.type)}
            aria-label={op.label}
            title={`${op.label} (${op.shortcut})`}
            className="inline-flex items-center justify-center h-7 w-7 rounded-lg transition-colors text-muted-foreground hover:bg-muted hover:text-foreground"
          >
            <Icon size={16} strokeWidth={1.5} />
          </button>
        );
      })}
    </div>
  );
}
