import ColorPicker from '@/components/shared/color-picker'
import NumberInput from '@/components/shared/number-input'
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
    <div className="space-y-2">
      <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
        Effects
      </h4>

      {!shadow ? (
        <button
          type="button"
          onClick={handleAddShadow}
          className="text-xs text-primary hover:text-primary/80"
        >
          + Add Shadow
        </button>
      ) : (
        <div className="space-y-2 bg-muted/30 rounded p-2">
          <div className="flex items-center justify-between">
            <span className="text-xs text-foreground">Shadow</span>
            <button
              type="button"
              onClick={handleRemoveShadow}
              className="text-xs text-muted-foreground hover:text-destructive"
            >
              Remove
            </button>
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
