/**
 * Generate indentation string.
 * @param depth - nesting depth
 * @param unit - indent unit (default: 2 spaces). Use '    ' for 4-space languages (Kotlin, Swift).
 */
export function indent(depth: number, unit = '  '): string {
  return unit.repeat(depth);
}
