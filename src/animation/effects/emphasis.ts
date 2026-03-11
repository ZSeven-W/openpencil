import { registerEffect } from '../effect-registry'

function kfId(): string {
  return `kf_${Math.random().toString(36).slice(2, 9)}`
}

// --- hold ---
registerEffect({
  id: 'hold',
  name: 'Hold',
  category: 'emphasis',
  properties: [],
  parameters: [],
  defaultDuration: 1000,
  generate: () => {
    return [
      { id: kfId(), offset: 0, properties: {}, easing: 'linear' },
      { id: kfId(), offset: 1, properties: {}, easing: 'linear' },
    ]
  },
})

// --- pulse ---
registerEffect({
  id: 'pulse',
  name: 'Pulse',
  category: 'emphasis',
  properties: ['opacity'],
  parameters: [],
  defaultDuration: 800,
  generate: ({ currentState }) => {
    const baseOpacity = currentState.opacity ?? 1
    return [
      { id: kfId(), offset: 0, properties: { opacity: baseOpacity }, easing: 'easeInOut' },
      { id: kfId(), offset: 0.5, properties: { opacity: 0.5 }, easing: 'easeInOut' },
      { id: kfId(), offset: 1, properties: { opacity: baseOpacity }, easing: 'linear' },
    ]
  },
})
