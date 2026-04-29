import { describe, it, expect } from 'vitest';
import type { PenDocument, VariableDefinition, ThemedValue } from '@zseven-w/pen-types';
import {
  applySemanticPalette,
  getSemanticPalette,
  getSemanticPaletteDescription,
  getSemanticPaletteHex,
  hasSemanticPalette,
  SEMANTIC_PALETTE_NAMES,
  resolveVariableRef,
  resolveColorRef,
  createEmptyDocument,
} from '../index.js';

/**
 * 14-variable semantic palette — the prerequisite for v1 theme-
 * aware element tools. See dark-theme audit note for the rationale:
 * `openpencil-docs/superpowers/notes/2026-04-22-dark-theme-defaults-
 * audit.md`.
 */

describe('getSemanticPalette', () => {
  it('total variable count matches SEMANTIC_PALETTE_NAMES length', () => {
    const p = getSemanticPalette();
    expect(Object.keys(p.variables).length).toBe(SEMANTIC_PALETTE_NAMES.length);
  });

  it('defines the Mode theme axis with Light + Dark', () => {
    const p = getSemanticPalette();
    expect(p.themes.Mode).toEqual(['Light', 'Dark']);
  });

  it('every light/dark color variable is type="color"', () => {
    const p = getSemanticPalette();
    // All color variables (light/dark or single) must have type "color"
    const colorVars = Object.entries(p.variables).filter(([, def]) =>
      (def as VariableDefinition).type === 'color'
    );
    expect(colorVars.length).toBeGreaterThanOrEqual(28);
  });

  it('light/dark color variables have ThemedValue[] with exactly light + dark entries', () => {
    const p = getSemanticPalette();
    // Only themed (array-value) variables must have the right shape
    for (const [name, def] of Object.entries(p.variables)) {
      const value = (def as VariableDefinition).value;
      if (!Array.isArray(value)) continue; // single-value entries are fine
      const arr = value as ThemedValue[];
      expect(arr.length, `${name} has 2 themed values`).toBe(2);
      expect(arr[0].theme).toEqual({ Mode: 'Light' });
      expect(arr[1].theme).toEqual({ Mode: 'Dark' });
    }
  });

  it('includes every required semantic role', () => {
    const p = getSemanticPalette();
    // The 14 required names from the audit doc
    const required = [
      'color-surface',
      'color-surface-2',
      'color-surface-3',
      'color-bg-deep',
      'color-border',
      'color-border-strong',
      'color-text-primary',
      'color-text-body',
      'color-text-muted',
      'color-text-subtle',
      'color-accent',
      'color-destructive',
      'color-success',
      'color-scrim',
    ];
    for (const name of required) {
      expect(p.variables[name], `missing: ${name}`).toBeDefined();
    }
  });

  it('light and dark values differ for every themed color (real theme support, not stub)', () => {
    const p = getSemanticPalette();
    for (const [name, def] of Object.entries(p.variables)) {
      const value = (def as VariableDefinition).value;
      if (!Array.isArray(value)) continue; // single-value entries skip
      const values = value as ThemedValue[];
      expect(values[0].value, `${name} has distinct light vs dark`).not.toBe(values[1].value);
    }
  });

  it('every color variable value is a well-formed hex color', () => {
    const p = getSemanticPalette();
    // Accept #RRGGBB (6) or #RRGGBBAA (8 — for scrims with alpha)
    const hexRe = /^#[0-9A-F]{6}([0-9A-F]{2})?$/i;
    for (const [name, def] of Object.entries(p.variables)) {
      const d = def as VariableDefinition;
      if (d.type !== 'color') continue;
      if (Array.isArray(d.value)) {
        const arr = d.value as ThemedValue[];
        for (const v of arr) {
          expect(typeof v.value).toBe('string');
          expect((v.value as string).match(hexRe), `${name} ${JSON.stringify(v.theme)}`).toBeTruthy();
        }
      } else {
        // single-value color
        expect(typeof d.value).toBe('string');
        expect((d.value as string).match(hexRe), `${name}`).toBeTruthy();
      }
    }
  });
});

describe('getSemanticPaletteHex', () => {
  it('default mode is Light', () => {
    const hex = getSemanticPaletteHex();
    expect(hex['color-surface']).toBe('#FFFFFF');
    expect(hex['color-accent']).toBe('#2563EB');
  });

  it('Light mode returns light hex values', () => {
    const hex = getSemanticPaletteHex('Light');
    expect(hex['color-bg-deep']).toBe('#F8FAFC');
    expect(hex['color-text-primary']).toBe('#0F172A');
  });

  it('Dark mode returns dark hex values', () => {
    const hex = getSemanticPaletteHex('Dark');
    expect(hex['color-surface']).toBe('#1E293B');
    expect(hex['color-bg-deep']).toBe('#0F172A');
    expect(hex['color-text-primary']).toBe('#F1F5F9');
    expect(hex['color-accent']).toBe('#60A5FA');
  });

  it('returns all color token names (omits numeric tokens)', () => {
    const hex = getSemanticPaletteHex();
    const p = getSemanticPalette();
    // Only color variables should appear in the hex map
    for (const [name, def] of Object.entries(p.variables)) {
      if ((def as VariableDefinition).type === 'color') {
        expect(hex[name], `${name} should be in hex map`).toBeDefined();
      } else {
        expect(hex[name], `${name} (numeric) should NOT be in hex map`).toBeUndefined();
      }
    }
  });
});

describe('getSemanticPaletteDescription', () => {
  it('returns a human-readable string for every variable', () => {
    for (const name of SEMANTIC_PALETTE_NAMES) {
      const desc = getSemanticPaletteDescription(name);
      expect(desc, `${name} description`).toBeTruthy();
      expect(desc!.length).toBeGreaterThan(5);
    }
  });

  it('returns undefined for unknown variable', () => {
    expect(getSemanticPaletteDescription('no-such-var')).toBeUndefined();
  });
});

describe('applySemanticPalette', () => {
  it('seeds an empty document with palette + theme axis', () => {
    const doc: PenDocument = createEmptyDocument();
    const out = applySemanticPalette(doc);
    expect(Object.keys(out.variables ?? {}).length).toBe(SEMANTIC_PALETTE_NAMES.length);
    expect(out.themes?.Mode).toEqual(['Light', 'Dark']);
  });

  it('does NOT mutate the input document', () => {
    const doc: PenDocument = createEmptyDocument();
    const before = JSON.stringify(doc);
    applySemanticPalette(doc);
    expect(JSON.stringify(doc)).toBe(before);
  });

  it('user-defined variables WIN on collision', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [],
      variables: {
        'color-accent': { type: 'color', value: '#FF0000' },
      },
    };
    const out = applySemanticPalette(doc);
    expect(out.variables!['color-accent'].value).toBe('#FF0000');
    // All other palette vars still added
    expect(out.variables!['color-surface']).toBeDefined();
    expect(Object.keys(out.variables!).length).toBe(SEMANTIC_PALETTE_NAMES.length);
  });

  it('user-defined theme axis WINS (does not overwrite existing Mode)', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [],
      themes: { Mode: ['High-Contrast', 'Normal'] },
    };
    const out = applySemanticPalette(doc);
    expect(out.themes!.Mode).toEqual(['High-Contrast', 'Normal']);
  });

  it('adds Mode axis if missing, preserves other axes', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [],
      themes: { Density: ['Compact', 'Comfortable'] },
    };
    const out = applySemanticPalette(doc);
    expect(out.themes!.Density).toEqual(['Compact', 'Comfortable']);
    expect(out.themes!.Mode).toEqual(['Light', 'Dark']);
  });

  it('applies cleanly to a doc without variables or themes', () => {
    const doc: PenDocument = { version: '1.0.0', children: [] };
    const out = applySemanticPalette(doc);
    expect(Object.keys(out.variables!).length).toBe(SEMANTIC_PALETTE_NAMES.length);
    expect(out.themes!.Mode).toEqual(['Light', 'Dark']);
  });
});

describe('hasSemanticPalette', () => {
  it('empty document → false', () => {
    expect(hasSemanticPalette(createEmptyDocument())).toBe(false);
  });

  it('after applySemanticPalette → true', () => {
    const doc = applySemanticPalette(createEmptyDocument());
    expect(hasSemanticPalette(doc)).toBe(true);
  });

  it('partial palette (missing one variable) → false', () => {
    const full = applySemanticPalette(createEmptyDocument());
    const partial = {
      ...full,
      variables: { ...full.variables },
    };
    delete (partial.variables as Record<string, unknown>)['color-accent'];
    expect(hasSemanticPalette(partial)).toBe(false);
  });
});

describe('palette resolves through resolveVariableRef', () => {
  it('Light theme: $color-accent → #2563EB', () => {
    const p = getSemanticPalette();
    const v = resolveVariableRef('$color-accent', p.variables, { Mode: 'Light' });
    expect(v).toBe('#2563EB');
  });

  it('Dark theme: $color-accent → #60A5FA', () => {
    const p = getSemanticPalette();
    const v = resolveVariableRef('$color-accent', p.variables, { Mode: 'Dark' });
    expect(v).toBe('#60A5FA');
  });

  it('no active theme: defaults to first (Light) value', () => {
    const p = getSemanticPalette();
    const v = resolveVariableRef('$color-accent', p.variables);
    expect(v).toBe('#2563EB');
  });

  it('resolveColorRef on a $ref produces the resolved hex (Dark)', () => {
    const p = getSemanticPalette();
    const hex = resolveColorRef('$color-surface', p.variables, { Mode: 'Dark' });
    expect(hex).toBe('#1E293B');
  });

  it('resolveColorRef on a non-ref passes through unchanged', () => {
    const p = getSemanticPalette();
    const hex = resolveColorRef('#CAFE00', p.variables, { Mode: 'Dark' });
    expect(hex).toBe('#CAFE00');
  });

  it('every color palette variable resolves to its documented light/dark value', () => {
    const p = getSemanticPalette();
    const light = getSemanticPaletteHex('Light');
    const dark = getSemanticPaletteHex('Dark');
    // getSemanticPaletteHex omits numeric tokens, only test what's in the hex map
    for (const name of Object.keys(light)) {
      const lightResolved = resolveVariableRef(`$${name}`, p.variables, { Mode: 'Light' });
      const darkResolved = resolveVariableRef(`$${name}`, p.variables, { Mode: 'Dark' });
      expect(lightResolved, `${name} light`).toBe(light[name]);
      expect(darkResolved, `${name} dark`).toBe(dark[name]);
    }
  });
});

describe('SEMANTIC_PALETTE_NAMES', () => {
  it('is readonly array matching all palette keys', () => {
    const p = getSemanticPalette();
    expect(SEMANTIC_PALETTE_NAMES.length).toBe(Object.keys(p.variables).length);
    for (const name of SEMANTIC_PALETTE_NAMES) {
      expect(p.variables[name]).toBeDefined();
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 1.1.5 — Alert + chart color tokens (14 new)
// ─────────────────────────────────────────────────────────────────────────────

describe('alert tokens (8 light/dark pairs)', () => {
  const ALERT_TOKENS = [
    'color-info-bg',
    'color-info-text',
    'color-success-bg',
    'color-success-text',
    'color-warning-bg',
    'color-warning-text',
    'color-danger-bg',
    'color-danger-text',
  ] as const;

  it('all 8 alert tokens are present in getSemanticPalette()', () => {
    const p = getSemanticPalette();
    for (const name of ALERT_TOKENS) {
      expect(p.variables[name], `missing: ${name}`).toBeDefined();
    }
  });

  it('alert tokens are type="color" with light + dark ThemedValue[]', () => {
    const p = getSemanticPalette();
    for (const name of ALERT_TOKENS) {
      const def = p.variables[name] as VariableDefinition;
      expect(def.type, `${name} type`).toBe('color');
      const arr = def.value as ThemedValue[];
      expect(Array.isArray(arr), `${name} value is array`).toBe(true);
      expect(arr.length, `${name} has 2 themed values`).toBe(2);
      expect(arr[0].theme).toEqual({ Mode: 'Light' });
      expect(arr[1].theme).toEqual({ Mode: 'Dark' });
    }
  });

  it('color-info-bg: light=#DBEAFE dark=#1E3A8A', () => {
    const p = getSemanticPalette();
    const arr = (p.variables['color-info-bg'] as VariableDefinition).value as ThemedValue[];
    expect(arr[0].value).toBe('#DBEAFE');
    expect(arr[1].value).toBe('#1E3A8A');
  });

  it('color-warning-text: light=#92400E dark=#FDE68A', () => {
    const p = getSemanticPalette();
    const arr = (p.variables['color-warning-text'] as VariableDefinition)
      .value as ThemedValue[];
    expect(arr[0].value).toBe('#92400E');
    expect(arr[1].value).toBe('#FDE68A');
  });

  it('color-danger-bg: light=#FEE2E2 dark=#7F1D1D', () => {
    const p = getSemanticPalette();
    const arr = (p.variables['color-danger-bg'] as VariableDefinition).value as ThemedValue[];
    expect(arr[0].value).toBe('#FEE2E2');
    expect(arr[1].value).toBe('#7F1D1D');
  });
});

describe('chart color tokens (6 single-value)', () => {
  const CHART_TOKENS = [
    'color-chart-1',
    'color-chart-2',
    'color-chart-3',
    'color-chart-4',
    'color-chart-5',
    'color-chart-6',
  ] as const;

  const CHART_EXPECTED: Record<string, string> = {
    'color-chart-1': '#3B82F6',
    'color-chart-2': '#8B5CF6',
    'color-chart-3': '#EC4899',
    'color-chart-4': '#14B8A6',
    'color-chart-5': '#F59E0B',
    'color-chart-6': '#F97316',
  };

  it('all 6 chart tokens present in getSemanticPalette()', () => {
    const p = getSemanticPalette();
    for (const name of CHART_TOKENS) {
      expect(p.variables[name], `missing: ${name}`).toBeDefined();
    }
  });

  it('chart tokens are type="color" with a plain string value (no theme axis)', () => {
    const p = getSemanticPalette();
    for (const name of CHART_TOKENS) {
      const def = p.variables[name] as VariableDefinition;
      expect(def.type, `${name} type`).toBe('color');
      expect(typeof def.value, `${name} value is string`).toBe('string');
    }
  });

  it('chart token values match spec', () => {
    const p = getSemanticPalette();
    for (const [name, expected] of Object.entries(CHART_EXPECTED)) {
      const def = p.variables[name] as VariableDefinition;
      expect(def.value, `${name} value`).toBe(expected);
    }
  });

  it('getSemanticPaletteHex includes chart tokens as-is (same for Light and Dark)', () => {
    const light = getSemanticPaletteHex('Light');
    const dark = getSemanticPaletteHex('Dark');
    for (const [name, expected] of Object.entries(CHART_EXPECTED)) {
      expect(light[name], `${name} light`).toBe(expected);
      expect(dark[name], `${name} dark`).toBe(expected);
    }
  });
});

describe('palette count snapshot (color additions complete)', () => {
  it('palette has exactly 32 color-type variables', () => {
    const p = getSemanticPalette();
    const colorCount = Object.values(p.variables).filter(
      (d) => (d as VariableDefinition).type === 'color',
    ).length;
    expect(colorCount).toBe(32);
  });

  it('SEMANTIC_PALETTE_NAMES matches total variable count', () => {
    const p = getSemanticPalette();
    expect(SEMANTIC_PALETTE_NAMES.length).toBe(Object.keys(p.variables).length);
  });

  it('applySemanticPalette seeds all palette variables on an empty document', () => {
    const doc: PenDocument = createEmptyDocument();
    const out = applySemanticPalette(doc);
    expect(Object.keys(out.variables ?? {}).length).toBe(Object.keys(getSemanticPalette().variables).length);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 1.2 — Typography tokens: size + weight + line-height × 6 roles (18)
// ─────────────────────────────────────────────────────────────────────────────

describe('typography tokens (18 numeric single-value)', () => {
  const TYPE_TOKENS: Record<string, number> = {
    'type-display-size': 64,
    'type-display-weight': 700,
    'type-display-line-height': 1.0,
    'type-h1-size': 24,
    'type-h1-weight': 600,
    'type-h1-line-height': 1.2,
    'type-h2-size': 20,
    'type-h2-weight': 600,
    'type-h2-line-height': 1.25,
    'type-h3-size': 16,
    'type-h3-weight': 600,
    'type-h3-line-height': 1.3,
    'type-body-size': 14,
    'type-body-weight': 400,
    'type-body-line-height': 1.5,
    'type-caption-size': 12,
    'type-caption-weight': 400,
    'type-caption-line-height': 1.4,
  };

  it('all 18 typography tokens present in getSemanticPalette()', () => {
    const p = getSemanticPalette();
    for (const name of Object.keys(TYPE_TOKENS)) {
      expect(p.variables[name], `missing: ${name}`).toBeDefined();
    }
  });

  it('typography tokens have type="number"', () => {
    const p = getSemanticPalette();
    for (const name of Object.keys(TYPE_TOKENS)) {
      const def = p.variables[name] as VariableDefinition;
      expect(def.type, `${name} type`).toBe('number');
    }
  });

  it('typography token values match spec', () => {
    const p = getSemanticPalette();
    for (const [name, expected] of Object.entries(TYPE_TOKENS)) {
      const def = p.variables[name] as VariableDefinition;
      expect(def.value, `${name}`).toBe(expected);
    }
  });

  it('typography tokens are NOT included in getSemanticPaletteHex()', () => {
    const hex = getSemanticPaletteHex('Light');
    for (const name of Object.keys(TYPE_TOKENS)) {
      expect(hex[name], `${name} should not appear in hex map`).toBeUndefined();
    }
  });

  it('palette has at least 50 tokens (32 color + 18 type)', () => {
    const p = getSemanticPalette();
    expect(Object.keys(p.variables).length).toBeGreaterThanOrEqual(50);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task 1.3 — Sparse letterSpacing tokens (2)
// ─────────────────────────────────────────────────────────────────────────────

describe('letterSpacing tokens (2 sparse numeric)', () => {
  it('type-display-letter-spacing present with value -0.5', () => {
    const p = getSemanticPalette();
    const def = p.variables['type-display-letter-spacing'] as VariableDefinition;
    expect(def, 'missing type-display-letter-spacing').toBeDefined();
    expect(def.type).toBe('number');
    expect(def.value).toBe(-0.5);
  });

  it('type-uppercase-label-letter-spacing present with value 1.5', () => {
    const p = getSemanticPalette();
    const def = p.variables['type-uppercase-label-letter-spacing'] as VariableDefinition;
    expect(def, 'missing type-uppercase-label-letter-spacing').toBeDefined();
    expect(def.type).toBe('number');
    expect(def.value).toBe(1.5);
  });

  it('palette total grows to 52 (50 + 2 letterSpacing)', () => {
    const p = getSemanticPalette();
    expect(Object.keys(p.variables).length).toBe(52);
  });
});
