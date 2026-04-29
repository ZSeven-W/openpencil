import { describe, it, expect } from 'vitest';
import { resolveVariableRef, resolveNodeForCanvas } from '../variables/resolve.js';
import { createEmptyDocument } from '../tree-utils.js';
import type { PenNode } from '@zseven-w/pen-types';

describe('resolver-side fallback (un-seeded doc, P1.6)', () => {
  const emptyVars = {};

  describe('color tokens fall back to default light/dark hex', () => {
    it('color ref no theme → light hex', () => {
      expect(resolveVariableRef('$color-text-primary', emptyVars, undefined)).toBe('#0F172A');
    });
    it('color ref Mode: Light → light hex', () => {
      expect(resolveVariableRef('$color-text-primary', emptyVars, { Mode: 'Light' })).toBe(
        '#0F172A',
      );
    });
    it('color ref Mode: Dark → dark hex', () => {
      expect(resolveVariableRef('$color-text-primary', emptyVars, { Mode: 'Dark' })).toBe(
        '#F1F5F9',
      );
    });
    it('alert color ref Mode: Light', () => {
      expect(resolveVariableRef('$color-info-bg', emptyVars, { Mode: 'Light' })).toBe('#DBEAFE');
    });
    it('alert color ref Mode: Dark', () => {
      expect(resolveVariableRef('$color-info-bg', emptyVars, { Mode: 'Dark' })).toBe('#1E3A8A');
    });
    it('chart color ref (single value, theme-agnostic)', () => {
      expect(resolveVariableRef('$color-chart-1', emptyVars, undefined)).toBe('#3B82F6');
      expect(resolveVariableRef('$color-chart-1', emptyVars, { Mode: 'Dark' })).toBe('#3B82F6');
    });
  });

  describe('typography tokens fall back to default numeric values', () => {
    it('size ref → number', () => {
      expect(resolveVariableRef('$type-h1-size', emptyVars, undefined)).toBe(24);
    });
    it('weight ref → number', () => {
      expect(resolveVariableRef('$type-h1-weight', emptyVars, undefined)).toBe(600);
    });
    it('line-height ref → number', () => {
      expect(resolveVariableRef('$type-h1-line-height', emptyVars, undefined)).toBe(1.2);
    });
    it('letterSpacing ref → number', () => {
      expect(resolveVariableRef('$type-display-letter-spacing', emptyVars, undefined)).toBe(-0.5);
    });
  });

  describe('spacing tokens fall back to default numeric values', () => {
    it('spacing-3 → 12', () => {
      expect(resolveVariableRef('$spacing-3', emptyVars, undefined)).toBe(12);
    });
    it('spacing-5 → 24', () => {
      expect(resolveVariableRef('$spacing-5', emptyVars, undefined)).toBe(24);
    });
  });

  describe('radius tokens fall back to default numeric values', () => {
    it('radius-md → 8', () => {
      expect(resolveVariableRef('$radius-md', emptyVars, undefined)).toBe(8);
    });
  });

  describe('un-known refs still return undefined', () => {
    it('non-existent ref → undefined', () => {
      expect(resolveVariableRef('$nonexistent-token', emptyVars, undefined)).toBeUndefined();
    });
  });

  describe('seeded vars take precedence over fallback', () => {
    it('user-set ref overrides default', () => {
      const vars = {
        'color-text-primary': { type: 'color' as const, value: '#FF0000' },
      };
      expect(resolveVariableRef('$color-text-primary', vars, undefined)).toBe('#FF0000');
    });
  });
});

describe('GAP fixes — resolveNodeForCanvas covers all numeric refs', () => {
  it('resolves fontWeight ref via fallback', () => {
    const doc = createEmptyDocument();
    const textNode: PenNode = {
      id: 'h',
      type: 'text',
      name: 'H1',
      content: 'Hello',
      fontSize: '$type-h1-size' as unknown as number,
      fontWeight: '$type-h1-weight' as unknown as number,
      fill: [{ type: 'solid', color: '$color-text-primary' }],
    };
    const resolved = resolveNodeForCanvas(
      textNode,
      doc.variables ?? {},
      undefined,
    ) as unknown as Record<string, unknown>;
    expect(typeof resolved.fontWeight).toBe('number');
    expect(resolved.fontWeight).toBe(600);
  });

  it('resolves refs in empty-vars doc (fallback fires past early-exit)', () => {
    // createEmptyDocument() returns doc.variables = undefined; passing undefined as vars
    // exercises the GAP-2 fix (the early-exit was `if (!variables || Object.keys...)`).
    const textNode: PenNode = {
      id: 't',
      type: 'text',
      name: 'T',
      content: 'Hi',
      fill: [{ type: 'solid', color: '$color-text-primary' }],
    };
    const resolved = resolveNodeForCanvas(
      textNode,
      undefined as unknown as Record<string, import('@zseven-w/pen-types').VariableDefinition>,
      undefined,
    ) as unknown as Record<string, unknown>;
    const fill = (resolved.fill as Array<{ type: string; color: string }>)[0];
    expect(fill.color).toBe('#0F172A');
  });
});
