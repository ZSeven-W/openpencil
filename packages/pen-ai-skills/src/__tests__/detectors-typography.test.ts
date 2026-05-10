import { describe, it, expect } from 'vitest';
import type { PenNode, PenDocument } from '@zseven-w/pen-types';
import { detectTextBgContrast } from '../diagnostics/detectors-typography';

// 2026-05-09 user reported black-card-with-white-text mixed into a cream
// page — the white-on-cream paths read as "muddy" and the AA check is the
// systematic way to flag it. Auto-fix is intentionally omitted; the right
// replacement depends on the design system + theme so this stays detect-only.

const text = (
  id: string,
  fill: unknown,
  fontSize: number = 16,
  fontWeight?: number | string,
): PenNode =>
  ({
    id,
    type: 'text',
    content: 'sample',
    fontSize,
    fontWeight,
    fill,
  }) as unknown as PenNode;

const frame = (id: string, children: PenNode[], fill?: unknown): PenNode =>
  ({
    id,
    type: 'frame',
    layout: 'vertical',
    fill,
    children,
  }) as unknown as PenNode;

const solid = (color: string) => [{ type: 'solid' as const, color }];

const emptyDoc: PenDocument = {
  children: [],
  variables: undefined,
  themes: undefined,
} as unknown as PenDocument;

const docWithVars = (vars: Record<string, unknown>, themes?: Record<string, string[]>) =>
  ({
    children: [],
    variables: vars,
    themes,
  }) as unknown as PenDocument;

describe('detectTextBgContrast', () => {
  it('does NOT flag black text on white background (ratio 21:1)', () => {
    const root = frame('page', [text('t1', solid('#000000'))], solid('#FFFFFF'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('flags very-low-contrast gray on white (ratio ~2.1 < default 2.5)', () => {
    // 2026-05-10 calibration — was #888888 (ratio 3.95) when threshold was
    // WCAG AA 4.5; corpus replay showed the strict threshold flagging
    // industry-standard caption patterns. Lowered to 2.5 normal / 2.0 large
    // so only genuinely-broken contrast trips. #B0B0B0 is the lighter
    // boundary still failing 2.5.
    const root = frame('page', [text('t1', solid('#B0B0B0'))], solid('#FFFFFF'));
    const issues = detectTextBgContrast(root, emptyDoc);
    expect(issues).toHaveLength(1);
    expect(issues[0].nodeId).toBe('t1');
    expect(issues[0].category).toBe('text-bg-contrast');
    expect(issues[0].severity).toBe('info');
    expect(issues[0].suggestedValue).toBeNull();
    expect(issues[0].reason).toMatch(/below 2\.5:1/);
  });

  it('does NOT flag Tailwind slate-400 captions on white (ratio ~2.56 — intentional tertiary text)', () => {
    // The 2026-05-08 corpus replay was 43% noise because WCAG-AA strict
    // flagged this pattern. New threshold tolerates it.
    const root = frame('page', [text('t1', solid('#94A3B8'))], solid('#FFFFFF'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('does NOT flag Tailwind blue-600 chips on blue-100 (ratio ~4.24 — chip pattern)', () => {
    const root = frame('page', [text('t1', solid('#2563EB'))], solid('#DBEAFE'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('flags white text on white bg (ratio 1.0 — invisible)', () => {
    const root = frame('page', [text('t1', solid('#FFFFFF'))], solid('#FFFFFF'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(1);
  });

  it('does NOT flag white text on dark cream-styled card (the user-reported good case)', () => {
    // Cream page with a dark card; white text on the dark card should pass.
    const root = frame(
      'page',
      [frame('card', [text('t1', solid('#FFFFFF'))], solid('#1A1A1A'))],
      solid('#FFF8E7'), // cream
    );
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('flags white text on cream page (the user-reported bad case)', () => {
    // The same white text but with no dark card wrapping — sits directly
    // on the cream page background; ratio ~1.10, fails AA hard.
    const root = frame('page', [text('t1', solid('#FFFFFF'))], solid('#FFF8E7'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(1);
  });

  it('uses the LARGE-text threshold (2.0) for fontSize >= 24', () => {
    // #B0B0B0 on white = ratio ~2.13 — fails normal 2.5 but passes large 2.0
    const root = frame('page', [text('t1', solid('#B0B0B0'), 32)], solid('#FFFFFF'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('uses the LARGE-text threshold (2.0) for fontSize >= 19 + bold weight', () => {
    const root = frame('page', [text('t1', solid('#B0B0B0'), 20, 700)], solid('#FFFFFF'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('still flags >=19px non-bold text (large rule needs 700+ weight)', () => {
    // Non-bold large text uses the NORMAL threshold (2.5); 2.13 < 2.5 → flag.
    const root = frame('page', [text('t1', solid('#B0B0B0'), 20, 400)], solid('#FFFFFF'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(1);
  });

  it('honors caller-supplied opts.normalThreshold to enforce stricter audits', () => {
    // 2.56:1 (slate-400) is silenced by default 2.5 but should re-fire when
    // a stricter audit asks for WCAG-AA 4.5.
    const root = frame('page', [text('t1', solid('#94A3B8'))], solid('#FFFFFF'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
    expect(detectTextBgContrast(root, emptyDoc, { normalThreshold: 4.5 })).toHaveLength(1);
  });

  it('walks ancestor chain to find first non-transparent bg', () => {
    // Outer page has cream; inner section has no fill (transparent);
    // the text's effective bg should still resolve to cream.
    const root = frame(
      'page',
      [
        frame('section', [text('t1', solid('#FFF8E7'))]), // section has no fill
      ],
      solid('#FFF8E7'),
    );
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(1);
  });

  it('defaults to white bg when no ancestor has any fill', () => {
    // No page fill at all — detector falls back to the canvas default
    // (white). White text on white = invisible.
    const root = frame('page', [text('t1', solid('#FFFFFF'))]);
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(1);
  });

  it('resolves $variable refs through doc.variables / theme', () => {
    const doc = docWithVars({
      'color-text': { type: 'color', value: '#B0B0B0' },
      'color-bg': { type: 'color', value: '#FFFFFF' },
    });
    const root = frame('page', [text('t1', solid('$color-text'))], solid('$color-bg'));
    const issues = detectTextBgContrast(root, doc);
    expect(issues).toHaveLength(1);
    expect(issues[0].reason).toMatch(/text=#B0B0B0 on bg=#FFFFFF/);
  });

  it('skips text whose color ref does not resolve (no false positive)', () => {
    const doc = docWithVars({}); // no variables, ref will not resolve
    const root = frame('page', [text('t1', solid('$nonexistent'))], solid('#FFFFFF'));
    expect(detectTextBgContrast(root, doc)).toHaveLength(0);
  });

  it('skips text with no fill array (renderer default applies)', () => {
    const root = frame('page', [text('t1', undefined)], solid('#FFFFFF'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('approximates gradient bg by first stop', () => {
    const gradientBg = [
      {
        type: 'linear_gradient' as const,
        stops: [
          { color: '#FFFFFF', offset: 0 },
          { color: '#000000', offset: 1 },
        ],
      },
    ];
    // First stop is white → light bg; white text fails.
    const root = frame('page', [text('t1', solid('#FFFFFF'))], gradientBg);
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(1);
  });

  // 2026-05-10 Codex stop-hook review caught: a wrapper with opacity=0
  // or #RRGGBBAA alpha=00 was being treated as the bg color, masking
  // contrast failures whose real bg lives further up the ancestor chain.

  it('skips wrapper fill with opacity=0 and uses real bg further up', () => {
    const transparentFill = [{ type: 'solid' as const, color: '#FFFFFF', opacity: 0 }];
    // Page is cream; transparent white wrapper between page and text.
    // Without the fix the detector would think bg=#FFFFFF (the wrapper)
    // and pass cream-text-on-white. With the fix it walks past the
    // invisible wrapper and flags cream-text-on-cream.
    const root = frame(
      'page',
      [frame('wrap', [text('t1', solid('#FFF8E7'))], transparentFill)],
      solid('#FFF8E7'),
    );
    const issues = detectTextBgContrast(root, emptyDoc);
    expect(issues).toHaveLength(1);
    expect(issues[0].reason).toMatch(/bg=#FFF8E7/);
  });

  it('skips wrapper fill with 8-hex alpha 00 and uses real bg further up', () => {
    const transparentFill = [{ type: 'solid' as const, color: '#FFFFFF00' }];
    const root = frame(
      'page',
      [frame('wrap', [text('t1', solid('#FFF8E7'))], transparentFill)],
      solid('#FFF8E7'),
    );
    const issues = detectTextBgContrast(root, emptyDoc);
    expect(issues).toHaveLength(1);
    expect(issues[0].reason).toMatch(/bg=#FFF8E7/);
  });

  // 2026-05-10 Codex round 4 — align the walk prune with pen-core's
  // canonical isNodeVisible (visible/enabled, NOT opacity). Renderer
  // treats opacity as paint alpha; pruning on opacity=0 would diverge
  // from what the renderer walks, so a contrast detector should match.
  // opacity=0 wrappers still get bypassed at the bg-resolution layer
  // because alpha-0 fills paint nothing visible.

  it('walks into opacity=0 subtree but uses real bg from behind the invisible wrapper', () => {
    const wrap: PenNode = {
      id: 'wrap',
      type: 'frame',
      layout: 'vertical',
      fill: solid('#FFFFFF'),
      opacity: 0,
      children: [text('t1', solid('#FFF8E7'))],
    } as unknown as PenNode;
    const root = frame('page', [wrap], solid('#FFF8E7'));
    const issues = detectTextBgContrast(root, emptyDoc);
    expect(issues).toHaveLength(1);
    expect(issues[0].reason).toMatch(/bg=#FFF8E7/);
  });

  it('does NOT flag text inside a visible=false subtree (canonical hidden)', () => {
    const wrap: PenNode = {
      id: 'wrap',
      type: 'frame',
      layout: 'vertical',
      fill: solid('#FFFFFF'),
      visible: false,
      children: [text('t1', solid('#FFF8E7'))],
    } as unknown as PenNode;
    const root = frame('page', [wrap], solid('#FFF8E7'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('does NOT flag text inside an enabled=false subtree (canonical hidden)', () => {
    const wrap: PenNode = {
      id: 'wrap',
      type: 'frame',
      layout: 'vertical',
      fill: solid('#FFFFFF'),
      enabled: false,
      children: [text('t1', solid('#FFF8E7'))],
    } as unknown as PenNode;
    const root = frame('page', [wrap], solid('#FFF8E7'));
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('still treats opacity=0.5 as visible (only opacity=0 is the alpha-0 sentinel)', () => {
    // We don't try to math a 50% wash against the layer below; that is
    // outside the detector's scope. The fill stays as the bg.
    const halfFill = [{ type: 'solid' as const, color: '#000000', opacity: 0.5 }];
    const root = frame('page', [text('t1', solid('#FFFFFF'))], halfFill);
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('does NOT skip 8-hex with alpha 80 (semi-transparent stays opaque)', () => {
    // #00000080 is 50% black — out of scope for the detector, but it is
    // NOT the same as alpha=00. Treat it as an opaque-enough bg.
    const semi = [{ type: 'solid' as const, color: '#00000080' }];
    const root = frame('page', [text('t1', solid('#FFFFFF'))], semi);
    expect(detectTextBgContrast(root, emptyDoc)).toHaveLength(0);
  });

  it('flags multiple offending texts in one pass', () => {
    const root = frame(
      'page',
      [
        text('good', solid('#000000')),
        text('bad-1', solid('#FFFFFF')),
        text('bad-2', solid('#EEEEEE')),
      ],
      solid('#FFFFFF'),
    );
    const issues = detectTextBgContrast(root, emptyDoc);
    expect(issues).toHaveLength(2);
    expect(new Set(issues.map((i) => i.nodeId))).toEqual(new Set(['bad-1', 'bad-2']));
  });
});
