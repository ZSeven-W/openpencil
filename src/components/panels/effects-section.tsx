import ColorPicker from '@/components/shared/color-picker'
import NumberInput from '@/components/shared/number-input'
import SectionHeader from '@/components/shared/section-header'
import { Button } from '@/components/ui/button'
import { Plus, Minus } from 'lucide-react'
import type { PenNode } from '@/types/pen'
import type { PenEffect, ShadowEffect } from '@/types/styles'

interface EffectsSectionProps {
  effects?: PenEffect[]
  onUpdate: (updates: Partial<PenNode>) => void
}

function findShadow(effects?: PenEffect[]): ShadowEffect | undefined {
  return effects?.find((e): e is ShadowEffect => e.type === 'shadow')
}

export default function EffectsSection({
  effects,
  onUpdate,
}: EffectsSectionProps) {
  const shadow = findShadow(effects)

  const handleAddShadow = () => {
    const current = effects ?? []
    const newEffect: ShadowEffect = {
      type: 'shadow',
      offsetX: 4,
      offsetY: 4,
      blur: 8,
      spread: 0,
      color: 'rgba(0,0,0,0.25)',
    }
    onUpdate({
      effects: [...current, newEffect],
    } as Partial<PenNode>)
  }

  const handleRemoveShadow = () => {
    const current = effects ?? []
    onUpdate({
      effects: current.filter((e) => e.type !== 'shadow'),
    } as Partial<PenNode>)
  }

  const handleUpdateShadow = (updates: Partial<ShadowEffect>) => {
    if (!shadow || !effects) return
    const newEffects = effects.map((e) => {
      if (e.type === 'shadow') return { ...e, ...updates }
      return e
    })
    onUpdate({ effects: newEffects } as Partial<PenNode>)
  }

  return (
    <div className="space-y-1.5">
      <SectionHeader
        title="Effects"
        actions={
          !shadow ? (
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={handleAddShadow}
            >
              <Plus className="w-3.5 h-3.5" />
            </Button>
          ) : undefined
        }
      />

      {shadow && (
        <div className="space-y-1 bg-secondary/50 rounded p-1.5">
          <div className="flex items-center justify-between h-5">
            <span className="text-[11px] text-foreground">
              Drop shadow
            </span>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={handleRemoveShadow}
              className="h-5 w-5"
            >
              <Minus className="w-3 h-3" />
            </Button>
          </div>

          <div className="grid grid-cols-2 gap-1">
            <NumberInput
              label="X"
              value={shadow.offsetX}
              onChange={(v) => handleUpdateShadow({ offsetX: v })}
            />
            <NumberInput
              label="Y"
              value={shadow.offsetY}
              onChange={(v) => handleUpdateShadow({ offsetY: v })}
            />
            <NumberInput
              label="Blur"
              value={shadow.blur}
              onChange={(v) => handleUpdateShadow({ blur: v })}
              min={0}
            />
            <NumberInput
              label="Spread"
              value={shadow.spread}
              onChange={(v) => handleUpdateShadow({ spread: v })}
              min={0}
            />
          </div>

          <ColorPicker
            label="Color"
            value={shadow.color}
            onChange={(c) => handleUpdateShadow({ color: c })}
          />
        </div>
      )}
    </div>
  )
}
