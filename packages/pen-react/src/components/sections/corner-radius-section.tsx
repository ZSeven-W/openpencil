import { NumberInput } from '../number-input.js';
import type { PenNode } from '@zseven-w/pen-types';

interface CornerRadiusSectionProps {
  cornerRadius?: number | [number, number, number, number];
  onUpdate: (updates: Partial<PenNode>) => void;
}

export function CornerRadiusSection({ cornerRadius, onUpdate }: CornerRadiusSectionProps) {
  const value =
    typeof cornerRadius === 'number'
      ? cornerRadius
      : Array.isArray(cornerRadius)
        ? cornerRadius[0]
        : 0;

  return (
    <div className="space-y-2">
      <h4 className="text-xs font-medium text-muted-foreground tracking-wider">Corner Radius</h4>
      <NumberInput
        value={value}
        onChange={(v) => onUpdate({ cornerRadius: v } as Partial<PenNode>)}
        min={0}
        max={999}
      />
    </div>
  );
}
