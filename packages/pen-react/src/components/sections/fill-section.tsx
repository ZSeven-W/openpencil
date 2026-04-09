import { ColorPicker } from '../color-picker.js';
import { NumberInput } from '../number-input.js';
import { SectionHeader } from '../section-header.js';
import { VariablePicker } from '../variable-picker.js';
import { Plus, X, Image as ImageIcon } from 'lucide-react';
import { isVariableRef } from '@zseven-w/pen-core';
import type { PenNode } from '@zseven-w/pen-types';
import type { PenFill, GradientStop } from '@zseven-w/pen-types';

const FILL_TYPE_OPTIONS = [
  { value: 'solid', label: 'Solid' },
  { value: 'linear_gradient', label: 'Linear Gradient' },
  { value: 'radial_gradient', label: 'Radial Gradient' },
  { value: 'image', label: 'Image' },
];

function defaultStops(): GradientStop[] {
  return [
    { offset: 0, color: '#000000' },
    { offset: 1, color: '#ffffff' },
  ];
}

function gradientPreviewCss(fill: PenFill): string | undefined {
  if (fill.type === 'linear_gradient') {
    const angle = fill.angle ?? 0;
    const stops = fill.stops.map((s) => `${s.color} ${Math.round(s.offset * 100)}%`).join(', ');
    return `linear-gradient(${angle}deg, ${stops})`;
  }
  if (fill.type === 'radial_gradient') {
    const stops = fill.stops.map((s) => `${s.color} ${Math.round(s.offset * 100)}%`).join(', ');
    return `radial-gradient(circle, ${stops})`;
  }
  return undefined;
}

interface FillSectionProps {
  fills?: PenFill[];
  onUpdate: (updates: Partial<PenNode>) => void;
}

export function FillSection({ fills, onUpdate }: FillSectionProps) {
  const safeFills: PenFill[] | undefined =
    typeof fills === 'string'
      ? [{ type: 'solid', color: fills }]
      : Array.isArray(fills)
        ? fills.map((f) => (typeof f === 'string' ? { type: 'solid' as const, color: f } : f))
        : undefined;
  const firstFill = safeFills?.[0];
  const fillType = firstFill?.type ?? 'solid';

  const currentColor = firstFill?.type === 'solid' ? firstFill.color : '#d1d5db';
  const currentAngle = firstFill?.type === 'linear_gradient' ? (firstFill.angle ?? 0) : 0;
  const currentStops: GradientStop[] =
    firstFill && (firstFill.type === 'linear_gradient' || firstFill.type === 'radial_gradient')
      ? firstFill.stops
      : defaultStops();
  const fillOpacity =
    firstFill && 'opacity' in firstFill ? Math.round((firstFill.opacity ?? 1) * 100) : 100;

  const handleTypeChange = (type: string) => {
    let newFills: PenFill[];
    if (type === 'solid') {
      newFills = [{ type: 'solid', color: currentColor }];
    } else if (type === 'linear_gradient') {
      newFills = [{ type: 'linear_gradient', angle: currentAngle, stops: currentStops }];
    } else if (type === 'radial_gradient') {
      newFills = [{ type: 'radial_gradient', cx: 0.5, cy: 0.5, radius: 0.5, stops: currentStops }];
    } else {
      newFills = [{ type: 'image', url: '' }];
    }
    onUpdate({ fill: newFills } as Partial<PenNode>);
  };

  const handleColorChange = (color: string) => {
    onUpdate({ fill: [{ type: 'solid', color }] } as Partial<PenNode>);
  };

  const handleOpacityChange = (val: number) => {
    if (!firstFill) return;
    const opacity = Math.max(0, Math.min(100, val)) / 100;
    onUpdate({ fill: [{ ...firstFill, opacity }] } as Partial<PenNode>);
  };

  const handleAngleChange = (angle: number) => {
    if (firstFill?.type === 'linear_gradient') {
      onUpdate({ fill: [{ ...firstFill, angle }] } as Partial<PenNode>);
    }
  };

  const handleStopColorChange = (index: number, color: string) => {
    if (
      !firstFill ||
      (firstFill.type !== 'linear_gradient' && firstFill.type !== 'radial_gradient')
    )
      return;
    const newStops = [...firstFill.stops];
    newStops[index] = { ...newStops[index], color };
    onUpdate({ fill: [{ ...firstFill, stops: newStops }] } as Partial<PenNode>);
  };

  const handleStopOffsetChange = (index: number, offset: number) => {
    if (
      !firstFill ||
      (firstFill.type !== 'linear_gradient' && firstFill.type !== 'radial_gradient')
    )
      return;
    const newStops = [...firstFill.stops];
    newStops[index] = { ...newStops[index], offset: offset / 100 };
    onUpdate({ fill: [{ ...firstFill, stops: newStops }] } as Partial<PenNode>);
  };

  const handleAddStop = () => {
    if (
      !firstFill ||
      (firstFill.type !== 'linear_gradient' && firstFill.type !== 'radial_gradient')
    )
      return;
    const stops = [...firstFill.stops];
    const lastOffset = stops[stops.length - 1]?.offset ?? 0.5;
    stops.push({ offset: Math.min(1, lastOffset + 0.1), color: '#888888' });
    onUpdate({ fill: [{ ...firstFill, stops }] } as Partial<PenNode>);
  };

  const handleRemoveStop = (index: number) => {
    if (
      !firstFill ||
      (firstFill.type !== 'linear_gradient' && firstFill.type !== 'radial_gradient')
    )
      return;
    if (firstFill.stops.length <= 2) return;
    const stops = firstFill.stops.filter((_, i) => i !== index);
    onUpdate({ fill: [{ ...firstFill, stops }] } as Partial<PenNode>);
  };

  const handleRemoveFill = () => {
    onUpdate({ fill: [] } as Partial<PenNode>);
  };

  const gradientCss = firstFill ? gradientPreviewCss(firstFill) : undefined;

  return (
    <div className="space-y-1.5">
      <SectionHeader
        title="Fill"
        actions={
          <button
            type="button"
            onClick={() => handleTypeChange('solid')}
            className="inline-flex items-center justify-center h-6 w-6 rounded text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
        }
      />

      {firstFill && (
        <div className="flex items-center gap-1.5 h-7">
          {fillType === 'solid' && !isVariableRef(currentColor) && (
            <div
              className="w-5 h-5 rounded border border-border shrink-0 cursor-pointer"
              style={{ backgroundColor: currentColor }}
            />
          )}
          {(fillType === 'linear_gradient' || fillType === 'radial_gradient') && gradientCss && (
            <div
              className="w-5 h-5 rounded border border-border shrink-0"
              style={{ background: gradientCss }}
            />
          )}
          {fillType === 'image' && (
            <div className="w-5 h-5 rounded border border-border shrink-0 bg-muted flex items-center justify-center">
              <ImageIcon className="w-3 h-3 text-muted-foreground" />
            </div>
          )}

          <select
            value={fillType}
            onChange={(e) => handleTypeChange(e.target.value)}
            className="h-6 text-[11px] flex-1 min-w-0 bg-secondary border border-transparent rounded px-1 focus:outline-none focus:border-ring text-foreground"
          >
            {FILL_TYPE_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>

          <NumberInput
            value={fillOpacity}
            onChange={handleOpacityChange}
            min={0}
            max={100}
            suffix="%"
            className="w-14"
          />

          <button
            type="button"
            onClick={handleRemoveFill}
            className="inline-flex items-center justify-center h-6 w-6 rounded text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
          >
            <X className="w-3 h-3" />
          </button>
        </div>
      )}

      {fillType === 'solid' && (
        <div className="flex items-center gap-1">
          <div className="flex-1">
            {isVariableRef(currentColor) ? (
              <div className="h-6 flex items-center px-2 bg-secondary rounded text-[11px] font-mono text-muted-foreground">
                {currentColor}
              </div>
            ) : (
              <ColorPicker value={currentColor} onChange={handleColorChange} />
            )}
          </div>
          <VariablePicker
            type="color"
            currentValue={currentColor}
            onBind={(ref) =>
              onUpdate({ fill: [{ type: 'solid', color: ref }] } as Partial<PenNode>)
            }
            onUnbind={(val) =>
              onUpdate({ fill: [{ type: 'solid', color: String(val) }] } as Partial<PenNode>)
            }
          />
        </div>
      )}

      {(fillType === 'linear_gradient' || fillType === 'radial_gradient') && (
        <div className="space-y-1.5">
          {fillType === 'linear_gradient' && (
            <NumberInput
              label="Angle"
              value={currentAngle}
              onChange={handleAngleChange}
              min={0}
              max={360}
              suffix="°"
            />
          )}

          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <span className="text-[10px] text-muted-foreground">Stops</span>
              <button
                type="button"
                onClick={handleAddStop}
                className="inline-flex items-center justify-center h-5 w-5 rounded text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
              >
                <Plus className="w-3 h-3" />
              </button>
            </div>
            {currentStops.map((stop, i) => (
              <div key={i} className="flex items-center gap-1">
                <ColorPicker value={stop.color} onChange={(c) => handleStopColorChange(i, c)} />
                <NumberInput
                  value={Math.round(
                    (Number.isFinite(stop.offset)
                      ? stop.offset
                      : i / Math.max(currentStops.length - 1, 1)) * 100,
                  )}
                  onChange={(v) => handleStopOffsetChange(i, v)}
                  min={0}
                  max={100}
                  suffix="%"
                  className="w-[72px]"
                />
                {currentStops.length > 2 && (
                  <button
                    type="button"
                    onClick={() => handleRemoveStop(i)}
                    className="inline-flex items-center justify-center h-5 w-5 rounded text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
                  >
                    <X className="w-3 h-3" />
                  </button>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
