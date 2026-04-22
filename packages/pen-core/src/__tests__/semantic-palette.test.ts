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
  it('returns 14 variables', () => {
    const p = getSemanticPalette();
    expect(Object.keys(p.variables).length).toBe(14);
  });

  it('defines the Mode theme axis with Light + Dark', () => {
    const p = getSemanticPalette();
    expect(p.themes.Mode).toEqual(['Light', 'Dark']);
  });

  it('every variable is type="color"', () => {
    const p = getSemanticPalette();
    for (const [name, def] of Object.entries(p.variables)) {
      expect((def as VariableDefinition).type, `${name} type`).toBe('color');
    }
  });

  it('every variable has ThemedValue[] with exactly light + dark entries', () => {
    const p = getSemanticPalette();
    for (const [name, def] of Object.entries(p.variables)) {
      const value = (def as VariableDefinition).value;
      expect(Array.isArray(value), `${name} value is array`).toBe(true);
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

  it('light and dark values differ for every color (real theme support, not stub)', () => {
    const p = getSemanticPalette();
    for (const [name, def] of Object.entries(p.variables)) {
      const values = (def as VariableDefinition).value as ThemedValue[];
      expect(values[0].value, `${name} has distinct light vs dark`).not.toBe(values[1].value);
    }
  });

  it('every value is a well-formed hex color', () => {
    const p = getSemanticPalette();
    // Accept #RRGGBB (6) or #RRGGBBAA (8 — for scrims with alpha)
    const hexRe = /^#[0-9A-F]{6}([0-9A-F]{2})?$/i;
    for (const [name, def] of Object.entries(p.variables)) {
      const values = (def as VariableDefinition).value as ThemedValue[];
      for (const v of values) {
        expect(typeof v.value).toBe('string');
        expect((v.value as string).match(hexRe), `${name} ${JSON.stringify(v.theme)}`).toBeTruthy();
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

  it('returns all 14 names', () => {
    const hex = getSemanticPaletteHex();
    expect(Object.keys(hex).length).toBe(14);
    for (const name of SEMANTIC_PALETTE_NAMES) {
      expect(hex[name]).toBeDefined();
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
    expect(Object.keys(out.variables ?? {}).length).toBe(14);
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
    expect(Object.keys(out.variables!).length).toBe(14);
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
    expect(Object.keys(out.variables!).length).toBe(14);
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

  it('every palette variable resolves to its documented light/dark value', () => {
    const p = getSemanticPalette();
    const light = getSemanticPaletteHex('Light');
    const dark = getSemanticPaletteHex('Dark');
    for (const name of SEMANTIC_PALETTE_NAMES) {
      const lightResolved = resolveVariableRef(`$${name}`, p.variables, { Mode: 'Light' });
      const darkResolved = resolveVariableRef(`$${name}`, p.variables, { Mode: 'Dark' });
      expect(lightResolved, `${name} light`).toBe(light[name]);
      expect(darkResolved, `${name} dark`).toBe(dark[name]);
    }
  });
});

describe('SEMANTIC_PALETTE_NAMES', () => {
  it('is readonly 14-length array matching palette keys', () => {
    expect(SEMANTIC_PALETTE_NAMES.length).toBe(14);
    const p = getSemanticPalette();
    for (const name of SEMANTIC_PALETTE_NAMES) {
      expect(p.variables[name]).toBeDefined();
    }
  });
});
