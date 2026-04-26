import type { PenDocument, VariableDefinition } from '@zseven-w/pen-types';

/**
 * 14-variable semantic palette for theme-aware element tools.
 *
 * Inventory rationale lives in
 * `openpencil-docs/superpowers/notes/2026-04-22-dark-theme-defaults-audit.md`.
 * That doc surveys the 30 hex literals in v0 element builders and
 * clusters them into these 14 semantic tokens — enough coverage
 * for every theme-dependent surface, small enough to stay
 * memorable.
 *
 * Every variable ships with BOTH light and dark values keyed on
 * a single theme axis `Mode`. Callers who want a single-theme
 * document (no switching) can drop the `themes` field and the
 * resolver will pick the first (light) value.
 *
 * This module is PURELY declarative — it does NOT mutate
 * `createEmptyDocument()`'s default output. v0 element tools
 * keep emitting hex literals (v0 byte-parity contract). v1
 * tools will `resolveVariableRef('$color-surface', doc.variables,
 * doc.themes ? {Mode: 'Dark'} : undefined)` to pick the right
 * shade at build time.
 *
 * To apply the palette to an existing document:
 *
 *   ```ts
 *   const themed = applySemanticPalette(doc);
 *   ```
 *
 * To get the resolved hex values for a specific mode (useful for
 * tests and tools that need a concrete color without full
 * document context):
 *
 *   ```ts
 *   const light = getSemanticPaletteHex('Light');
 *   // { 'color-surface': '#FFFFFF', 'color-accent': '#2563EB', ... }
 *   ```
 */

export const SEMANTIC_PALETTE_THEME_AXIS = 'Mode' as const;
export const SEMANTIC_PALETTE_THEME_LIGHT = 'Light' as const;
export const SEMANTIC_PALETTE_THEME_DARK = 'Dark' as const;

export type SemanticPaletteMode =
  | typeof SEMANTIC_PALETTE_THEME_LIGHT
  | typeof SEMANTIC_PALETTE_THEME_DARK;

export interface SemanticPalette {
  /** The `Mode: ['Light', 'Dark']` theme axis. Merge into `doc.themes`. */
  themes: Record<string, string[]>;
  /** The 14 variable definitions. Merge into `doc.variables`. */
  variables: Record<string, VariableDefinition>;
}

/**
 * Raw light/dark value table. Source of truth — both
 * `getSemanticPalette()` and `getSemanticPaletteHex()` derive from
 * this. Keeping it as a plain object (not a function) makes the
 * palette trivially scanable in code review + reduces the risk of
 * light/dark getting out of sync inside a function body.
 */
const PALETTE: Record<string, { light: string; dark: string; description: string }> = {
  'color-surface': {
    light: '#FFFFFF',
    dark: '#1E293B',
    description: 'Primary surface — card, modal, tooltip background',
  },
  'color-surface-2': {
    light: '#F1F5F9',
    dark: '#334155',
    description: 'Secondary surface — chip, input, hover background',
  },
  'color-surface-3': {
    light: '#F3F4F6',
    dark: '#475569',
    description: 'Tertiary surface — pressed, deeper-nested background',
  },
  'color-bg-deep': {
    light: '#F8FAFC',
    dark: '#0F172A',
    description: 'Page background, skeleton hosts',
  },
  'color-border': {
    light: '#E2E8F0',
    dark: '#334155',
    description: 'Dividers, input strokes, card outlines',
  },
  'color-border-strong': {
    light: '#CBD5E1',
    dark: '#475569',
    description: 'Dashed chart-placeholder border',
  },
  'color-text-primary': {
    light: '#0F172A',
    dark: '#F1F5F9',
    description: 'Headlines, active page numbers',
  },
  'color-text-body': {
    light: '#334155',
    dark: '#CBD5E1',
    description: 'Body paragraphs, nav labels',
  },
  'color-text-muted': {
    light: '#64748B',
    dark: '#94A3B8',
    description: 'Secondary text, timestamps, placeholders',
  },
  'color-text-subtle': {
    light: '#94A3B8',
    dark: '#64748B',
    description: 'Tertiary text, disabled states',
  },
  'color-accent': {
    light: '#2563EB',
    dark: '#60A5FA',
    description: 'Primary brand, active pill, focus ring',
  },
  'color-destructive': {
    light: '#EF4444',
    dark: '#F87171',
    description: 'Delete actions, error state, trending-down',
  },
  'color-success': {
    light: '#10B981',
    dark: '#34D399',
    description: 'Trending-up, "Online" status',
  },
  'color-scrim': {
    light: '#00000080',
    dark: '#00000099',
    description: 'Modal backdrop',
  },
};

export const SEMANTIC_PALETTE_NAMES = Object.keys(PALETTE) as readonly (keyof typeof PALETTE)[];

/** Return the 14-variable palette as a `{themes, variables}` pair. */
export function getSemanticPalette(): SemanticPalette {
  const variables: Record<string, VariableDefinition> = {};
  for (const [name, { light, dark }] of Object.entries(PALETTE)) {
    variables[name] = {
      type: 'color',
      value: [
        { value: light, theme: { [SEMANTIC_PALETTE_THEME_AXIS]: SEMANTIC_PALETTE_THEME_LIGHT } },
        { value: dark, theme: { [SEMANTIC_PALETTE_THEME_AXIS]: SEMANTIC_PALETTE_THEME_DARK } },
      ],
    };
  }
  return {
    themes: {
      [SEMANTIC_PALETTE_THEME_AXIS]: [SEMANTIC_PALETTE_THEME_LIGHT, SEMANTIC_PALETTE_THEME_DARK],
    },
    variables,
  };
}

/**
 * Return just the resolved hex values for one mode. Useful when a
 * caller needs a concrete color without carrying a full PenDocument
 * context (e.g. test fixtures, static codegen).
 */
export function getSemanticPaletteHex(
  mode: SemanticPaletteMode = SEMANTIC_PALETTE_THEME_LIGHT,
): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [name, entry] of Object.entries(PALETTE)) {
    out[name] = mode === SEMANTIC_PALETTE_THEME_DARK ? entry.dark : entry.light;
  }
  return out;
}

/**
 * Return a human-readable description of a palette variable, for
 * tooling UI (token picker) and documentation generation.
 */
export function getSemanticPaletteDescription(name: string): string | undefined {
  return PALETTE[name]?.description;
}

/**
 * Merge the semantic palette into a PenDocument's `variables` and
 * `themes` fields. Returns a new document; does NOT mutate the
 * input. Existing variables/themes with the same names WIN — the
 * palette is purely additive when there's a collision, so a
 * document that has already defined `color-accent` differently
 * keeps its version.
 */
export function applySemanticPalette(doc: PenDocument): PenDocument {
  const palette = getSemanticPalette();
  const mergedVariables = { ...palette.variables, ...(doc.variables ?? {}) };
  const mergedThemes: Record<string, string[]> = { ...(doc.themes ?? {}) };
  for (const [axis, modes] of Object.entries(palette.themes)) {
    if (!mergedThemes[axis]) {
      mergedThemes[axis] = [...modes];
    }
  }
  return {
    ...doc,
    variables: mergedVariables,
    themes: mergedThemes,
  };
}

/**
 * Return `true` iff the document's variables include all 14
 * palette names. Useful as a runtime check before a v1 tool emits
 * `$color-*` refs — if the doc lacks the palette, the tool should
 * fall back to light-theme hex literals OR call
 * `applySemanticPalette()` to seed it.
 */
export function hasSemanticPalette(doc: PenDocument): boolean {
  const vars = doc.variables ?? {};
  return SEMANTIC_PALETTE_NAMES.every((name) => vars[name] !== undefined);
}
