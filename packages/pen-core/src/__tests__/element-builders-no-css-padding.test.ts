/**
 * Static guard: element-builder source files must not use the CSS-side
 * padding shorthands `paddingTop` / `paddingRight` / `paddingBottom`
 * / `paddingLeft` as object property assignments.
 *
 * The layout engine's `resolvePadding` (`packages/pen-core/src/layout/
 * engine.ts`) only reads the unified `padding` field
 * (`number | [T,R] | [T,R,B,L] | string`); the CSS-side siblings
 * are silently dropped, so nodes that declared them rendered with
 * zero insets. Codex caught this on `add_sidebar_nav_v0` (commit
 * 5c88aca6) and a 14-builder follow-up sweep landed in 5c91ff19.
 *
 * This test makes the trap loud — any future builder that re-introduces
 * the shorthand fails CI before reaching review. References to the
 * field names in JSDoc / line comments are still allowed (`* `, `//`)
 * because that's how the warning is documented in builder doc blocks.
 */

import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const BUILDERS_DIR = join(__dirname, '..', 'element-builders');

const FORBIDDEN = ['paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft'] as const;

function isCommentLine(line: string): boolean {
  const trimmed = line.trimStart();
  return (
    trimmed.startsWith('//') ||
    trimmed.startsWith('*') ||
    trimmed.startsWith('/*') ||
    trimmed.startsWith('*/')
  );
}

describe('element-builders — no CSS-side padding shorthands', () => {
  it('every builder uses padding: [T,R,B,L] / [T,R] / number, never paddingTop/Right/Bottom/Left', () => {
    const offenders: string[] = [];
    const files = readdirSync(BUILDERS_DIR).filter(
      (f) => f.endsWith('.ts') && !f.endsWith('.d.ts'),
    );
    for (const file of files) {
      const src = readFileSync(join(BUILDERS_DIR, file), 'utf-8');
      const lines = src.split('\n');
      for (let i = 0; i < lines.length; i += 1) {
        const line = lines[i];
        if (isCommentLine(line)) continue;
        for (const field of FORBIDDEN) {
          // Match `field:` as a property key (allow leading whitespace,
          // optional quotes). Avoids matching inside strings of comments
          // already filtered above; identifier boundary in front prevents
          // matching `notPaddingTop`.
          const re = new RegExp(`(?:^|[\\s,{('"\`])${field}\\s*:`);
          if (re.test(line)) {
            offenders.push(`${file}:${i + 1}: ${line.trim()}`);
          }
        }
      }
    }
    expect(
      offenders,
      `Builders using CSS-side padding shorthand silently render with 0 inset. ` +
        `Switch to the unified padding field (number | [T,R] | [T,R,B,L]).\n` +
        offenders.join('\n'),
    ).toEqual([]);
  });
});
