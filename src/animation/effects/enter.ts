import { registerEffect } from '../effect-registry'
import type { AnimatableValue } from '@/types/animation'

function kfId(): string {
  return `kf_${Math.random().toString(36).slice(2, 9)}`
}

// --- fade-in ---
registerEffect({
  id: 'fade-in',
  name: 'Fade In',
  category: 'enter',
  properties: ['opacity'],
  parameters: [],
  defaultDuration: 500,
  generate: ({ currentState }) => {
    return [
      { id: kfId(), offset: 0, properties: { opacity: 0 }, easing: 'easeOut' },
      { id: kfId(), offset: 1, properties: { opacity: currentState.opacity ?? 1 }, easing: 'linear' },
    ]
  },
})

// --- slide-in ---
registerEffect({
  id: 'slide-in',
  name: 'Slide In',
  category: 'enter',
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
    const targetX = (currentState.x as number) ?? 0
    const targetY = (currentState.y as number) ?? 0

    let startProps: Record<string, AnimatableValue>
    let endProps: Record<string, AnimatableValue>

    switch (direction) {
      case 'right':
        startProps = { x: targetX + 300 }
        endProps = { x: targetX }
        break
      case 'up':
        startProps = { y: targetY - 300 }
        endProps = { y: targetY }
        break
      case 'down':
        startProps = { y: targetY + 300 }
        endProps = { y: targetY }
        break
      case 'left':
      default:
        startProps = { x: targetX - 300 }
        endProps = { x: targetX }
        break
    }

    return [
      { id: kfId(), offset: 0, properties: startProps, easing: 'easeOut' },
      { id: kfId(), offset: 1, properties: endProps, easing: 'linear' },
    ]
  },
})

// --- scale-in ---
registerEffect({
  id: 'scale-in',
  name: 'Scale In',
  category: 'enter',
  properties: ['scaleX', 'scaleY'],
  parameters: [],
  defaultDuration: 500,
  generate: () => {
    return [
      { id: kfId(), offset: 0, properties: { scaleX: 0, scaleY: 0 }, easing: 'easeOut' },
      { id: kfId(), offset: 1, properties: { scaleX: 1, scaleY: 1 }, easing: 'linear' },
    ]
  },
})

// --- bounce-in ---
registerEffect({
  id: 'bounce-in',
  name: 'Bounce In',
  category: 'enter',
  properties: ['scaleX', 'scaleY'],
  parameters: [],
  defaultDuration: 600,
  generate: () => {
    return [
      { id: kfId(), offset: 0, properties: { scaleX: 0, scaleY: 0 }, easing: 'bouncy' },
      { id: kfId(), offset: 0.7, properties: { scaleX: 1.2, scaleY: 1.2 }, easing: 'easeOut' },
      { id: kfId(), offset: 1, properties: { scaleX: 1, scaleY: 1 }, easing: 'linear' },
    ]
  },
})

// --- blur-in ---
registerEffect({
  id: 'blur-in',
  name: 'Blur In',
  category: 'enter',
  properties: ['blur', 'opacity'],
  parameters: [],
  defaultDuration: 500,
  generate: ({ currentState }) => {
    return [
      { id: kfId(), offset: 0, properties: { blur: 20, opacity: 0 }, easing: 'easeOut' },
      { id: kfId(), offset: 1, properties: { blur: 0, opacity: currentState.opacity ?? 1 }, easing: 'linear' },
    ]
  },
})
