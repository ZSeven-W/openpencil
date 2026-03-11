/**
 * Bundled token presets as offline fallback.
 * Used when extraction fails or for quick-start design tokens.
 */

export interface TokenPreset {
  name: string
  colors: Record<string, string>
  fonts: { heading: string; body: string }
}

export const TOKEN_PRESETS: Record<string, TokenPreset> = {
  tailwind: {
    name: 'Tailwind CSS',
    colors: {
      primary: '#3b82f6',
      secondary: '#8b5cf6',
      accent: '#f59e0b',
      bg: '#ffffff',
      surface: '#f8fafc',
      text: '#0f172a',
      'text-muted': '#64748b',
      border: '#e2e8f0',
    },
    fonts: { heading: 'Inter, sans-serif', body: 'Inter, sans-serif' },
  },
  material: {
    name: 'Material Design',
    colors: {
      primary: '#6750a4',
      secondary: '#625b71',
      accent: '#7d5260',
      bg: '#fffbfe',
      surface: '#f7f2fa',
      text: '#1c1b1f',
      'text-muted': '#49454f',
      border: '#cac4d0',
    },
    fonts: { heading: 'Roboto, sans-serif', body: 'Roboto, sans-serif' },
  },
  github: {
    name: 'GitHub',
    colors: {
      primary: '#0969da',
      secondary: '#8250df',
      accent: '#bf8700',
      bg: '#ffffff',
      surface: '#f6f8fa',
      text: '#1f2328',
      'text-muted': '#656d76',
      border: '#d0d7de',
    },
    fonts: { heading: '-apple-system, BlinkMacSystemFont, sans-serif', body: '-apple-system, BlinkMacSystemFont, sans-serif' },
  },
  stripe: {
    name: 'Stripe',
    colors: {
      primary: '#635bff',
      secondary: '#0a2540',
      accent: '#00d4aa',
      bg: '#ffffff',
      surface: '#f6f9fc',
      text: '#0a2540',
      'text-muted': '#425466',
      border: '#e3e8ee',
    },
    fonts: { heading: '-apple-system, BlinkMacSystemFont, sans-serif', body: '-apple-system, BlinkMacSystemFont, sans-serif' },
  },
}

export const TOKEN_PRESET_NAMES: string[] = Object.keys(TOKEN_PRESETS)

export function getTokenPreset(name: string): TokenPreset | undefined {
  return TOKEN_PRESETS[name]
}
