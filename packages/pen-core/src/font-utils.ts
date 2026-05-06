// ---------------------------------------------------------------------------
// CSS 字体系列引用 — 出于可移植性而提取（无 CanvasKit deps）
// ---------------------------------------------------------------------------

const GENERIC_FAMILIES = new Set([
  'serif',
  'sans-serif',
  'monospace',
  'cursive',
  'fantasy',
  'system-ui',
  'ui-serif',
  'ui-sans-serif',
  'ui-monospace',
  'ui-rounded',
  '-apple-system',
  'blinkmacsystemfont',
]);

export function cssFontFamily(family: string): string {
  return family
    .split(',')
    .map((f) => {
      const trimmed = f.trim();
      if (!trimmed) return trimmed;
      // Already 引用
      if (
        (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
        (trimmed.startsWith("'") && trimmed.endsWith("'"))
      )
        return trimmed;
      // Generic 系列不得被引用
      if (GENERIC_FAMILIES.has(trimmed.toLowerCase())) return trimmed;
      // Quote 其他所有内容（即使对于单字名称也是安全的）
      return `"${trimmed}"`;
    })
    .join(', ');
}
