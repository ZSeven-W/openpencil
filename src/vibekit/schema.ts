/**
 * Vibe Kit Schema — the canonical contract between kits and templates.
 *
 * Every kit must define variables matching these names.
 * Every template references these `$variable` names.
 */

export type VibeCategory =
  | 'typography'
  | 'color'
  | 'texture'
  | 'lut'
  | 'sfx'
  | 'animation'
  | 'transition'
  | 'stroke'
  | 'size'
  | 'space'
  | 'graphic'

export interface VibeKitSchemaEntry {
  type: 'color' | 'number' | 'boolean' | 'string'
  fallback: string | number | boolean
  category: VibeCategory
  description?: string
}

export const VIBE_KIT_SCHEMA: Record<string, VibeKitSchemaEntry> = {
  // --- Typography ---
  'font-heading': { type: 'string', fallback: 'Inter, sans-serif', category: 'typography', description: 'Primary heading font stack' },
  'font-body': { type: 'string', fallback: 'Inter, sans-serif', category: 'typography', description: 'Body text font stack' },
  'font-editorial': { type: 'string', fallback: 'Georgia, serif', category: 'typography', description: 'Editorial/pull quote font stack' },
  'font-mono': { type: 'string', fallback: 'JetBrains Mono, monospace', category: 'typography', description: 'Monospace font stack' },

  // --- Colors ---
  'color-primary': { type: 'color', fallback: '#2563eb', category: 'color', description: 'Primary brand color' },
  'color-secondary': { type: 'color', fallback: '#7c3aed', category: 'color', description: 'Secondary brand color' },
  'color-accent': { type: 'color', fallback: '#f59e0b', category: 'color', description: 'Accent / highlight color' },
  'color-bg': { type: 'color', fallback: '#ffffff', category: 'color', description: 'Background color' },
  'color-surface': { type: 'color', fallback: '#f8fafc', category: 'color', description: 'Surface / card color' },
  'color-text': { type: 'color', fallback: '#0f172a', category: 'color', description: 'Primary text color' },
  'color-text-muted': { type: 'color', fallback: '#64748b', category: 'color', description: 'Muted / secondary text' },
  'color-border': { type: 'color', fallback: '#e2e8f0', category: 'color', description: 'Border color' },
  'color-success': { type: 'color', fallback: '#22c55e', category: 'color', description: 'Success / positive' },
  'color-warning': { type: 'color', fallback: '#f59e0b', category: 'color', description: 'Warning' },
  'color-error': { type: 'color', fallback: '#ef4444', category: 'color', description: 'Error / negative' },

  // --- Textures ---
  'texture-bg-1': { type: 'string', fallback: '', category: 'texture', description: 'Background texture URL' },
  'texture-pattern-1': { type: 'string', fallback: '', category: 'texture', description: 'Pattern texture URL' },

  // --- LUT Filters ---
  'lut-warm': { type: 'string', fallback: '', category: 'lut', description: 'Warm color grading preset URL' },
  'lut-cool': { type: 'string', fallback: '', category: 'lut', description: 'Cool color grading preset URL' },
  'lut-vintage': { type: 'string', fallback: '', category: 'lut', description: 'Vintage color grading preset URL' },

  // --- SFX ---
  'sfx-whoosh': { type: 'string', fallback: '', category: 'sfx', description: 'Whoosh sound effect URL' },
  'sfx-pop': { type: 'string', fallback: '', category: 'sfx', description: 'Pop sound effect URL' },
  'sfx-click': { type: 'string', fallback: '', category: 'sfx', description: 'Click sound effect URL' },

  // --- Animations ---
  'anim-enter': { type: 'string', fallback: 'fade-in', category: 'animation', description: 'Default enter animation preset' },
  'anim-exit': { type: 'string', fallback: 'fade-out', category: 'animation', description: 'Default exit animation preset' },
  'anim-emphasis': { type: 'string', fallback: 'bounce', category: 'animation', description: 'Emphasis animation preset' },

  // --- Transitions ---
  'transition-slide': { type: 'string', fallback: 'slide-left', category: 'transition', description: 'Slide transition between pages' },
  'transition-fade': { type: 'string', fallback: 'crossfade', category: 'transition', description: 'Fade transition between pages' },

  // --- Strokes ---
  'stroke-default': { type: 'number', fallback: 1, category: 'stroke', description: 'Default stroke thickness' },
  'stroke-decorative': { type: 'number', fallback: 3, category: 'stroke', description: 'Decorative stroke thickness' },

  // --- Size Tokens ---
  'size-heading-xl': { type: 'number', fallback: 48, category: 'size', description: 'Extra large heading size' },
  'size-heading-lg': { type: 'number', fallback: 36, category: 'size', description: 'Large heading size' },
  'size-heading-md': { type: 'number', fallback: 28, category: 'size', description: 'Medium heading size' },
  'size-heading-sm': { type: 'number', fallback: 22, category: 'size', description: 'Small heading size' },
  'size-body': { type: 'number', fallback: 16, category: 'size', description: 'Body text size' },
  'size-caption': { type: 'number', fallback: 13, category: 'size', description: 'Caption text size' },
  'size-eyebrow': { type: 'number', fallback: 11, category: 'size', description: 'Eyebrow / overline text size' },
  'size-radius-sm': { type: 'number', fallback: 4, category: 'size', description: 'Small corner radius' },
  'size-radius-md': { type: 'number', fallback: 8, category: 'size', description: 'Medium corner radius' },
  'size-radius-lg': { type: 'number', fallback: 16, category: 'size', description: 'Large corner radius' },

  // --- Space Tokens ---
  'space-xs': { type: 'number', fallback: 4, category: 'space', description: 'Extra small spacing' },
  'space-sm': { type: 'number', fallback: 8, category: 'space', description: 'Small spacing' },
  'space-md': { type: 'number', fallback: 16, category: 'space', description: 'Medium spacing' },
  'space-lg': { type: 'number', fallback: 24, category: 'space', description: 'Large spacing' },
  'space-xl': { type: 'number', fallback: 40, category: 'space', description: 'Extra large spacing' },
  'space-2xl': { type: 'number', fallback: 64, category: 'space', description: 'Double extra large spacing' },

  // --- Graphic Sets ---
  'icon-set': { type: 'string', fallback: 'lucide', category: 'graphic', description: 'Active icon library name' },
}

/** All schema variable names (without $ prefix). */
export const VIBE_KIT_VARIABLE_NAMES = Object.keys(VIBE_KIT_SCHEMA)

/** Get schema entries for a given category. */
export function getSchemaByCategory(category: VibeCategory): Record<string, VibeKitSchemaEntry> {
  const result: Record<string, VibeKitSchemaEntry> = {}
  for (const [name, entry] of Object.entries(VIBE_KIT_SCHEMA)) {
    if (entry.category === category) result[name] = entry
  }
  return result
}

/** All categories in display order. */
export const VIBE_CATEGORIES: VibeCategory[] = [
  'color',
  'typography',
  'size',
  'space',
  'stroke',
  'animation',
  'transition',
  'texture',
  'lut',
  'sfx',
  'graphic',
]

/** Validate that a variables record satisfies the schema. Returns missing variable names. */
export function validateKitVariables(variables: Record<string, unknown>): string[] {
  return VIBE_KIT_VARIABLE_NAMES.filter((name) => !(name in variables))
}
