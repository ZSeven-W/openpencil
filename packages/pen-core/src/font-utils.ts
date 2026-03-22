// ---------------------------------------------------------------------------
// CSS font family quoting — extracted for portability (no CanvasKit deps)
// ---------------------------------------------------------------------------

const GENERIC_FAMILIES = new Set([
  'serif', 'sans-serif', 'monospace', 'cursive', 'fantasy', 'system-ui',
  'ui-serif', 'ui-sans-serif', 'ui-monospace', 'ui-rounded',
  '-apple-system', 'blinkmacsystemfont',
])

export function cssFontFamily(family: string): string {
  return family.split(',').map(f => {
    const trimmed = f.trim()
    if (!trimmed) return trimmed
    // Already quoted
    if ((trimmed.startsWith('"') && trimmed.endsWith('"')) ||
        (trimmed.startsWith("'") && trimmed.endsWith("'"))) return trimmed
    // Generic families must not be quoted
    if (GENERIC_FAMILIES.has(trimmed.toLowerCase())) return trimmed
    // Quote everything else (safe even for single-word names)
    return `"${trimmed}"`
  }).join(', ')
}
