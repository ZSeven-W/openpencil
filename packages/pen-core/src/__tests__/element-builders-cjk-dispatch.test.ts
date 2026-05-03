import { describe, it, expect } from 'vitest';
import {
  buildBodyText,
  buildHeading,
  buildListRow,
  buildCardRow,
  buildSectionHeader,
  buildToast,
  buildAlert,
  buildEmptyState,
  buildFormField,
  buildBreadcrumb,
  buildLink,
  buildStatGrid,
  detectCjkScript,
  cjkFontFamily,
} from '../element-builders/index.js';

/**
 * CJK font dispatch regression matrix. The repo's text-rules contract:
 *
 *   - heading: dispatch fontFamily per script → Noto Sans SC / JP / KR
 *     (Latin / Arabic / emoji → no fontFamily, renderer default)
 *   - body-text: ALWAYS Inter regardless of script (Inter has CJK
 *     glyphs via fallback font stack)
 *   - other text-carrying builders: don't dispatch CJK (sub-text
 *     children inherit the renderer default; we record this as the
 *     baseline so a future regression that *starts* dispatching in
 *     one of them would trip the test and force a deliberate choice)
 *
 * The matrix covers 4 heading levels × 5 scripts for heading + 5
 * scripts for body-text + spot checks for 8 other text builders.
 */

// Script fixtures covering the detection contract:
//   - zh = Han only → 'chinese'
//   - ja = hiragana/katakana (even if mixed with Han) → 'japanese'
//   - ko = hangul → 'korean'
//   - ar = Arabic (not CJK at all) → null
//   - emoji = only emoji → null
//   - latin = pure Latin → null
const SCRIPTS = {
  zh: '你好世界',
  ja: 'こんにちは世界',
  ko: '안녕하세요 세계',
  ar: 'مرحبا بالعالم',
  emoji: '🎉🚀✨',
  latin: 'Hello, world',
} as const;

describe('cjk-detect — primitive dispatch', () => {
  it('chinese: Han ideographs → "chinese"', () => {
    expect(detectCjkScript(SCRIPTS.zh)).toBe('chinese');
  });
  it('japanese: hiragana dominates even if mixed with Han', () => {
    expect(detectCjkScript(SCRIPTS.ja)).toBe('japanese');
  });
  it('korean: hangul → "korean"', () => {
    expect(detectCjkScript(SCRIPTS.ko)).toBe('korean');
  });
  it('arabic: no CJK blocks → null', () => {
    expect(detectCjkScript(SCRIPTS.ar)).toBe(null);
  });
  it('emoji-only: no CJK blocks → null', () => {
    expect(detectCjkScript(SCRIPTS.emoji)).toBe(null);
  });
  it('latin: no CJK blocks → null', () => {
    expect(detectCjkScript(SCRIPTS.latin)).toBe(null);
  });

  it('cjkFontFamily maps chinese → Noto Sans SC', () => {
    expect(cjkFontFamily('chinese')).toBe('Noto Sans SC');
  });
  it('cjkFontFamily maps japanese → Noto Sans JP', () => {
    expect(cjkFontFamily('japanese')).toBe('Noto Sans JP');
  });
  it('cjkFontFamily maps korean → Noto Sans KR', () => {
    expect(cjkFontFamily('korean')).toBe('Noto Sans KR');
  });
  it('cjkFontFamily on null → undefined (fallback to theme default)', () => {
    expect(cjkFontFamily(null)).toBeUndefined();
  });
});

describe('heading — CJK font dispatched per script × level', () => {
  const LEVELS = ['display', 'h1', 'h2', 'h3'] as const;
  const CJK_EXPECT: Record<string, string | undefined> = {
    zh: 'Noto Sans SC',
    ja: 'Noto Sans JP',
    ko: 'Noto Sans KR',
    ar: undefined, // Arabic → Latin preset, no fontFamily set
    emoji: undefined,
    latin: undefined,
  };
  for (const level of LEVELS) {
    for (const [script, content] of Object.entries(SCRIPTS)) {
      const expected = CJK_EXPECT[script];
      it(`${level} × ${script}: fontFamily === ${expected ?? '<unset>'}`, () => {
        const h = buildHeading({ content, level }) as Record<string, unknown>;
        if (expected === undefined) {
          expect(h.fontFamily).toBeUndefined();
        } else {
          expect(h.fontFamily).toBe(expected);
        }
      });
    }
  }
});

describe('heading — CJK preset uses non-negative letterSpacing + wider lineHeight', () => {
  // Defensive: CJK characters overlap visibly with negative letterSpacing,
  // and stretched lineHeight (1.3+) is required for readable CJK columns.
  // If someone edits the CJK_BASE table, this test catches a regression.
  it('CJK display keeps lineHeight >= 1.3 and omits letterSpacing', () => {
    const h = buildHeading({ content: SCRIPTS.zh, level: 'display' }) as Record<string, unknown>;
    expect(h.lineHeight).toBeGreaterThanOrEqual(1.3);
    // No negative letterSpacing for CJK (typography contract)
    if (typeof h.letterSpacing === 'number') {
      expect(h.letterSpacing).toBeGreaterThanOrEqual(0);
    }
  });

  it('Latin display keeps tight letterSpacing (-0.5) and tight lineHeight (1.0)', () => {
    const h = buildHeading({ content: SCRIPTS.latin, level: 'display' }) as Record<string, unknown>;
    expect(h.letterSpacing).toBe(-0.5);
    expect(h.lineHeight).toBeLessThanOrEqual(1.0);
  });
});

describe('body-text — always Inter regardless of script', () => {
  for (const [script, content] of Object.entries(SCRIPTS)) {
    it(`${script}: fontFamily === Inter (CJK fallback via font stack, not explicit dispatch)`, () => {
      const b = buildBodyText({ content }) as Record<string, unknown>;
      expect(b.fontFamily).toBe('Inter');
    });
  }
});

describe('other text builders — baseline: no CJK dispatch leaks through', () => {
  // These builders carry text but the current contract is NOT to
  // dispatch CJK on their sub-text children. This test anchors the
  // baseline so a future regression that starts dispatching (or a
  // new builder that should start dispatching) becomes visible.

  function findText(
    n: Record<string, unknown>,
    out: Array<Record<string, unknown>> = [],
  ): Array<Record<string, unknown>> {
    if (n.type === 'text') out.push(n);
    const children = (n.children as Array<Record<string, unknown>> | undefined) ?? [];
    for (const c of children) findText(c, out);
    return out;
  }

  it('list-row CJK title: sub-text has no fontFamily (uses renderer default)', () => {
    const r = buildListRow({ title: SCRIPTS.zh, subtitle: SCRIPTS.zh }) as Record<string, unknown>;
    const texts = findText(r);
    for (const t of texts) {
      expect(t.fontFamily).toBeUndefined();
    }
  });

  it('card-row CJK card titles: sub-text has no fontFamily', () => {
    const r = buildCardRow({
      items: [{ title: SCRIPTS.ja, subtitle: SCRIPTS.ja }],
    }) as Record<string, unknown>;
    const texts = findText(r);
    for (const t of texts) {
      expect(t.fontFamily).toBeUndefined();
    }
  });

  it('section-header CJK title: no fontFamily leak', () => {
    const r = buildSectionHeader({ title: SCRIPTS.ko }) as Record<string, unknown>;
    const texts = findText(r);
    for (const t of texts) {
      expect(t.fontFamily).toBeUndefined();
    }
  });

  it('toast CJK message: no fontFamily leak', () => {
    const r = buildToast({ message: SCRIPTS.zh }) as Record<string, unknown>;
    const texts = findText(r);
    for (const t of texts) {
      expect(t.fontFamily).toBeUndefined();
    }
  });

  it('alert CJK message: no fontFamily leak', () => {
    const r = buildAlert({ message: SCRIPTS.ja }) as Record<string, unknown>;
    const texts = findText(r);
    for (const t of texts) {
      expect(t.fontFamily).toBeUndefined();
    }
  });

  it('empty-state CJK title/subtitle/cta: no fontFamily leak', () => {
    const r = buildEmptyState({
      title: SCRIPTS.zh,
      subtitle: SCRIPTS.zh,
      cta_label: SCRIPTS.ja,
    }) as Record<string, unknown>;
    const texts = findText(r);
    for (const t of texts) {
      expect(t.fontFamily).toBeUndefined();
    }
  });

  it('form-field CJK label: no fontFamily leak', () => {
    const r = buildFormField({ label: SCRIPTS.ko, placeholder: SCRIPTS.ko }) as Record<
      string,
      unknown
    >;
    const texts = findText(r);
    for (const t of texts) {
      expect(t.fontFamily).toBeUndefined();
    }
  });

  it('breadcrumb CJK labels: no fontFamily leak', () => {
    const r = buildBreadcrumb({
      items: [{ label: SCRIPTS.zh }, { label: SCRIPTS.zh }],
    }) as Record<string, unknown>;
    const texts = findText(r);
    for (const t of texts) {
      expect(t.fontFamily).toBeUndefined();
    }
  });

  it('link CJK label: no fontFamily leak', () => {
    const r = buildLink({ label: SCRIPTS.ja }) as Record<string, unknown>;
    const texts = findText(r);
    for (const t of texts) {
      expect(t.fontFamily).toBeUndefined();
    }
  });

  it('stat-grid CJK values/labels: no fontFamily leak', () => {
    const r = buildStatGrid({
      items: [{ value: SCRIPTS.zh, label: SCRIPTS.zh }],
    }) as Record<string, unknown>;
    const texts = findText(r);
    for (const t of texts) {
      expect(t.fontFamily).toBeUndefined();
    }
  });
});
