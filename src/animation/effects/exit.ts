import { registerEffect } from '../effect-registry'
import type { AnimatableValue } from '@/types/animation'

function kfId(): string {
  return `kf_${Math.random().toString(36).slice(2, 9)}`
}

// --- fade-out ---
registerEffect({
  id: 'fade-out',
  name: 'Fade Out',
  category: 'exit',
  properties: ['opacity'],
  parameters: [],
  defaultDuration: 500,
  generate: ({ currentState }) => {
    return [
      { id: kfId(), offset: 0, properties: { opacity: currentState.opacity ?? 1 }, easing: 'easeOut' },
      { id: kfId(), offset: 1, properties: { opacity: 0 }, easing: 'linear' },
    ]
  },
})

// --- slide-out ---
registerEffect({
  id: 'slide-out',
  name: 'Slide Out',
  category: 'exit',
  properties: ['x', 'y'],
  parameters: [
    {
      key: 'direction',
      type: 'direction',
      default: 'left',
      label: 'Direction',
      options: [
        { label: 'Left', value: 'left' },
        { label: 'Right', value: 'right' },
        { label: 'Up', value: 'up' },
        { label: 'Down', value: 'down' },
      ],
    },
  ],
  defaultDuration: 500,
  generate: ({ params, currentState }) => {
    const direction = params.direction as string
    const currentX = (currentState.x as number) ?? 0
    const currentY = (currentState.y as number) ?? 0

    let startProps: Record<string, AnimatableValue>
    let endProps: Record<string, AnimatableValue>

    switch (direction) {
      case 'right':
        startProps = { x: currentX }
        endProps = { x: currentX + 300 }
        break
      case 'up':
        startProps = { y: currentY }
        endProps = { y: currentY - 300 }
        break
      case 'down':
        startProps = { y: currentY }
        endProps = { y: currentY + 300 }
        break
      case 'left':
      default:
        startProps = { x: currentX }
        endProps = { x: currentX - 300 }
        break
    }

    return [
      { id: kfId(), offset: 0, properties: startProps, easing: 'easeOut' },
      { id: kfId(), offset: 1, properties: endProps, easing: 'linear' },
    ]
  },
})

// --- scale-out ---
registerEffect({
  id: 'scale-out',
  name: 'Scale Out',
  category: 'exit',
  properties: ['scaleX', 'scaleY'],
  parameters: [],
  defaultDuration: 500,
  generate: () => {
    return [
      { id: kfId(), offset: 0, properties: { scaleX: 1, scaleY: 1 }, easing: 'easeOut' },
      { id: kfId(), offset: 1, properties: { scaleX: 0, scaleY: 0 }, easing: 'linear' },
    ]
  },
})

// --- blur-out ---
registerEffect({
  id: 'blur-out',
  name: 'Blur Out',
  category: 'exit',
  properties: ['blur', 'opacity'],
  parameters: [],
  defaultDuration: 500,
  generate: ({ currentState }) => {
    return [
      { id: kfId(), offset: 0, properties: { blur: 0, opacity: currentState.opacity ?? 1 }, easing: 'easeOut' },
      { id: kfId(), offset: 1, properties: { blur: 20, opacity: 0 }, easing: 'linear' },
    ]
  },
})
