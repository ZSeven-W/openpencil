import { useTranslation } from 'react-i18next';
import NumberInput from '@/components/shared/number-input';
import SectionHeader from '@/components/shared/section-header';
import VariablePicker from '@/components/shared/variable-picker';
import { isVariableRef } from '@/variables/resolve-variables';
import type { PenNode, PolygonNode, EllipseNode } from '@/types/pen';

interface AppearanceSectionProps {
  node: PenNode;
  onUpdate: (updates: Partial<PenNode>) => void;
}

export default function AppearanceSection({ node, onUpdate }: AppearanceSectionProps) {
  const { t } = useTranslation();
  const rawOpacity = node.opacity;
  const isBound = typeof rawOpacity === 'string' && isVariableRef(rawOpacity);
  const opacity = typeof rawOpacity === 'number' ? rawOpacity * 100 : 100;
  const isPolygon = node.type === 'polygon';
  const isEllipse = node.type === 'ellipse';

  return (
    <div className="space-y-1.5">
      <SectionHeader title={t('appearance.layer')} />
      <div className="grid grid-cols-2 gap-1">
        <div className="flex items-center gap-1">
          <div className="flex-1">
            {isBound ? (
              <div className="h-6 flex items-center px-2 bg-secondary rounded text-[11px] font-mono text-muted-foreground">
                {rawOpacity}
              </div>
            ) : (
              <NumberInput
                label={t('appearance.opacity')}
                value={Math.round(opacity)}
                onChange={(v) => onUpdate({ opacity: v / 100 })}
                min={0}
                max={100}
                suffix="%"
              />
            )}
          </div>
          <VariablePicker
            type="number"
            currentValue={isBound ? String(rawOpacity) : undefined}
            onBind={(ref) => onUpdate({ opacity: ref as unknown as number })}
            onUnbind={(val) => onUpdate({ opacity: Number(val) })}
          />
        </div>
        {isPolygon && (
          <NumberInput
            label={t('polygon.sides')}
            value={(node as PolygonNode).polygonCount ?? 3}
            onChange={(v) => onUpdate({ polygonCount: v } as Partial<PenNode>)}
            min={3}
            max={100}
          />
        )}
      </div>
      {isEllipse && (
        <>
          <div className="grid grid-cols-3 gap-1 mt-1">
            <NumberInput
              label={t('ellipse.start')}
              value={Math.round((node as EllipseNode).startAngle ?? 0)}
              onChange={(v) => onUpdate({ startAngle: v } as Partial<PenNode>)}
              min={0}
              max={360}
              suffix="°"
            />
            <NumberInput
              label={t('ellipse.sweep')}
              value={Math.round((node as EllipseNode).sweepAngle ?? 360)}
              onChange={(v) => onUpdate({ sweepAngle: v } as Partial<PenNode>)}
              min={0}
              max={360}
              suffix="°"
            />
            <NumberInput
              label={t('ellipse.innerRadius')}
              value={Math.round(((node as EllipseNode).innerRadius ?? 0) * 100)}
              onChange={(v) => onUpdate({ innerRadius: v / 100 } as Partial<PenNode>)}
              min={0}
              max={99}
              suffix="%"
            />
          </div>
        </>
      )}
    </div>
  );
}
