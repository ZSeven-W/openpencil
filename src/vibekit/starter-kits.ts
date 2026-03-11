import type { VibeKit } from '@/types/vibekit'
import type { VariableDefinition } from '@/types/variables'
import { VIBE_KIT_SCHEMA } from './schema'

type VariableOverrides = Record<string, string | number | boolean>

/**
 * Build a complete VibeKit from partial overrides.
 * All schema variables are included — overrides replace fallback values.
 */
function buildKit(
  id: string,
  name: string,
  description: string,
  overrides: VariableOverrides,
): VibeKit {
  const variables: Record<string, VariableDefinition> = {}

  for (const [varName, entry] of Object.entries(VIBE_KIT_SCHEMA)) {
    variables[varName] = {
      type: entry.type,
      value: varName in overrides ? overrides[varName] : entry.fallback,
    }
  }

  return {
    id,
    name,
    description,
    version: '1.0.0',
    variables,
    assets: {},
    metadata: {
      createdAt: '2026-03-11T00:00:00Z',
      generatedBy: 'manual',
    },
  }
}

// ---------------------------------------------------------------------------
// 1. Corporate
// ---------------------------------------------------------------------------
const corporate = buildKit('kit-corporate', 'Corporate', 'Clean blues with Inter font', {
  'font-heading': 'Inter, sans-serif',
  'font-body': 'Inter, sans-serif',
  'font-editorial': 'Georgia, serif',
  'color-primary': '#1e40af',
  'color-secondary': '#3b82f6',
  'color-accent': '#0ea5e9',
  'color-bg': '#ffffff',
  'color-surface': '#f1f5f9',
  'color-text': '#0f172a',
  'color-text-muted': '#64748b',
  'color-border': '#cbd5e1',
  'size-heading-xl': 44,
  'size-heading-lg': 32,
  'size-body': 16,
  'space-md': 16,
  'space-lg': 24,
  'size-radius-md': 8,
})

// ---------------------------------------------------------------------------
// 2. Creative
// ---------------------------------------------------------------------------
const creative = buildKit('kit-creative', 'Creative', 'Purples and pinks with Playfair Display', {
  'font-heading': 'Playfair Display, serif',
  'font-body': 'DM Sans, sans-serif',
  'font-editorial': 'Playfair Display, serif',
  'color-primary': '#7c3aed',
  'color-secondary': '#ec4899',
  'color-accent': '#f472b6',
  'color-bg': '#faf5ff',
  'color-surface': '#f3e8ff',
  'color-text': '#1e1b4b',
  'color-text-muted': '#6b7280',
  'color-border': '#e9d5ff',
  'size-heading-xl': 52,
  'size-heading-lg': 38,
  'size-body': 17,
  'space-md': 20,
  'space-lg': 32,
  'space-xl': 48,
  'size-radius-md': 12,
  'size-radius-lg': 24,
})

// ---------------------------------------------------------------------------
// 3. Minimal
// ---------------------------------------------------------------------------
const minimal = buildKit('kit-minimal', 'Minimal', 'Black and white with system fonts', {
  'font-heading': 'system-ui, sans-serif',
  'font-body': 'system-ui, sans-serif',
  'font-editorial': 'Georgia, serif',
  'color-primary': '#18181b',
  'color-secondary': '#52525b',
  'color-accent': '#a1a1aa',
  'color-bg': '#ffffff',
  'color-surface': '#fafafa',
  'color-text': '#09090b',
  'color-text-muted': '#71717a',
  'color-border': '#e4e4e7',
  'size-heading-xl': 40,
  'size-heading-lg': 30,
  'size-body': 15,
  'space-md': 12,
  'space-lg': 20,
  'space-xl': 32,
  'size-radius-sm': 2,
  'size-radius-md': 4,
  'size-radius-lg': 8,
  'stroke-default': 1,
  'stroke-decorative': 1,
})

// ---------------------------------------------------------------------------
// 4. Bold
// ---------------------------------------------------------------------------
const bold = buildKit('kit-bold', 'Bold', 'High contrast with Poppins', {
  'font-heading': 'Poppins, sans-serif',
  'font-body': 'Poppins, sans-serif',
  'font-editorial': 'Poppins, sans-serif',
  'color-primary': '#dc2626',
  'color-secondary': '#f97316',
  'color-accent': '#facc15',
  'color-bg': '#0a0a0a',
  'color-surface': '#171717',
  'color-text': '#fafafa',
  'color-text-muted': '#a3a3a3',
  'color-border': '#404040',
  'size-heading-xl': 56,
  'size-heading-lg': 42,
  'size-heading-md': 32,
  'size-body': 18,
  'space-md': 20,
  'space-lg': 32,
  'space-xl': 48,
  'size-radius-md': 12,
  'size-radius-lg': 20,
  'stroke-default': 2,
  'stroke-decorative': 4,
})

// ---------------------------------------------------------------------------
// 5. Editorial
// ---------------------------------------------------------------------------
const editorial = buildKit('kit-editorial', 'Editorial', 'Warm tones with Lora serif headings', {
  'font-heading': 'Lora, serif',
  'font-body': 'Source Sans 3, sans-serif',
  'font-editorial': 'Lora, serif',
  'color-primary': '#92400e',
  'color-secondary': '#b45309',
  'color-accent': '#d97706',
  'color-bg': '#fffbeb',
  'color-surface': '#fef3c7',
  'color-text': '#1c1917',
  'color-text-muted': '#78716c',
  'color-border': '#e7e5e4',
  'size-heading-xl': 46,
  'size-heading-lg': 34,
  'size-body': 17,
  'size-caption': 14,
  'space-md': 18,
  'space-lg': 28,
  'space-xl': 44,
  'size-radius-sm': 2,
  'size-radius-md': 6,
  'size-radius-lg': 12,
})

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

export const STARTER_KITS: VibeKit[] = [corporate, creative, minimal, bold, editorial]

export function getStarterKitById(id: string): VibeKit | undefined {
  return STARTER_KITS.find((k) => k.id === id)
}
