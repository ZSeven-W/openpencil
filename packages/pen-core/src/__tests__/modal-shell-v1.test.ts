import { describe, it, expect } from 'vitest';
import {
  applySemanticPalette,
  buildModalShell,
  buildModalShellV1,
  createEmptyDocument,
  resolveColorRef,
} from '../index.js';

/**
 * add_modal_shell_v1 — the first theme-aware v1 element tool.
 *
 * Validates the v0 byte-parity contract AND the three theme
 * variants work end-to-end (including the round-trip through
 * resolveColorRef for `'system'` mode).
 */

interface Frame {
  type?: string;
  role?: string;
  fill?: Array<{ type: string; color: string }>;
  children?: Frame[];
}

function findByRole(n: Frame, role: string): Frame | undefined {
  if (n.role === role) return n;
  for (const c of n.children ?? []) {
    const hit = findByRole(c, role);
    if (hit) return hit;
  }
  return undefined;
}

function getFillColor(n: Frame): string | undefined {
  return n.fill?.[0]?.color;
}

describe('buildModalShellV1', () => {
  describe('v0 byte-parity on default (theme omitted or `light`)', () => {
    it('omitted theme → card fill #FFFFFF (matches v0)', () => {
      const v0 = buildModalShell({
        title: 'Delete?',
        subtitle: 'Cannot undo.',
      }) as unknown as Frame;
      const v1 = buildModalShellV1({
        title: 'Delete?',
        subtitle: 'Cannot undo.',
      }) as unknown as Frame;
      const v0Card = findByRole(v0, 'modal-shell-card')!;
      const v1Card = findByRole(v1, 'modal-shell-card')!;
      expect(getFillColor(v1Card)).toBe(getFillColor(v0Card));
      expect(getFillColor(v1Card)).toBe('#FFFFFF');
    });

    it('theme=light → subtitle fill #64748B (matches v0)', () => {
      const v0 = buildModalShell({ title: 'T', subtitle: 'S' }) as unknown as Frame;
      const v1 = buildModalShellV1({
        title: 'T',
        subtitle: 'S',
        theme: 'light',
      }) as unknown as Frame;
      const v0Sub = findByRole(v0, 'modal-subtitle')!;
      const v1Sub = findByRole(v1, 'modal-subtitle')!;
      expect(getFillColor(v1Sub)).toBe(getFillColor(v0Sub));
      expect(getFillColor(v1Sub)).toBe('#64748B');
    });

    it('theme=light title has NO fill (v0 parity — title was unstyled)', () => {
      const v0 = buildModalShell({ title: 'T' }) as unknown as Frame;
      const v1 = buildModalShellV1({ title: 'T', theme: 'light' }) as unknown as Frame;
      const v0Title = findByRole(v0, 'modal-title')!;
      const v1Title = findByRole(v1, 'modal-title')!;
      expect(v0Title.fill).toBeUndefined();
      expect(v1Title.fill).toBeUndefined();
    });

    it('structural shape matches v0 (same roles, same nesting)', () => {
      const v0 = buildModalShell({ title: 'T', subtitle: 'S' });
      const v1 = buildModalShellV1({ title: 'T', subtitle: 'S' });
      // Stringify without ids so structural match is checkable
      const stripIds = (obj: unknown): unknown => {
        if (Array.isArray(obj)) return obj.map(stripIds);
        if (obj && typeof obj === 'object') {
          const out: Record<string, unknown> = {};
          for (const [k, v] of Object.entries(obj)) {
            if (k === 'id') continue;
            out[k] = stripIds(v);
          }
          return out;
        }
        return obj;
      };
      // Names differ because v1 sets title fill to undefined in
      // the param slot but v0 never declares one; structurally
      // identical otherwise.
      expect(stripIds(v0)).toEqual(stripIds(v1));
    });
  });

  describe('theme=dark — hardcoded dark hex values', () => {
    it('card fill is dark-theme surface (#1E293B)', () => {
      const t = buildModalShellV1({ title: 'T', theme: 'dark' }) as unknown as Frame;
      const card = findByRole(t, 'modal-shell-card')!;
      expect(getFillColor(card)).toBe('#1E293B');
    });

    it('title fill is dark-theme primary text (#F1F5F9)', () => {
      const t = buildModalShellV1({ title: 'Delete?', theme: 'dark' }) as unknown as Frame;
      const title = findByRole(t, 'modal-title')!;
      expect(getFillColor(title)).toBe('#F1F5F9');
    });

    it('subtitle fill is dark-theme muted text (#94A3B8)', () => {
      const t = buildModalShellV1({
        title: 'T',
        subtitle: 'S',
        theme: 'dark',
      }) as unknown as Frame;
      const sub = findByRole(t, 'modal-subtitle')!;
      expect(getFillColor(sub)).toBe('#94A3B8');
    });

    it('scrim stays black in dark mode (intentional — backdrops are always dark)', () => {
      const t = buildModalShellV1({ title: 'T', theme: 'dark' }) as unknown as Frame;
      const scrim = findByRole(t, 'modal-scrim')!;
      expect(getFillColor(scrim)).toBe('#000000');
    });
  });

  describe('theme=system — emits $color-* refs', () => {
    it('card fill is $color-surface ref', () => {
      const t = buildModalShellV1({ title: 'T', theme: 'system' }) as unknown as Frame;
      const card = findByRole(t, 'modal-shell-card')!;
      expect(getFillColor(card)).toBe('$color-surface');
    });

    it('title fill is $color-text-primary ref', () => {
      const t = buildModalShellV1({ title: 'T', theme: 'system' }) as unknown as Frame;
      const title = findByRole(t, 'modal-title')!;
      expect(getFillColor(title)).toBe('$color-text-primary');
    });

    it('subtitle fill is $color-text-muted ref', () => {
      const t = buildModalShellV1({
        title: 'T',
        subtitle: 'S',
        theme: 'system',
      }) as unknown as Frame;
      const sub = findByRole(t, 'modal-subtitle')!;
      expect(getFillColor(sub)).toBe('$color-text-muted');
    });
  });

  describe('end-to-end: system theme round-trips through applySemanticPalette + resolveColorRef', () => {
    it('Light mode: $color-surface resolves to #FFFFFF', () => {
      const doc = applySemanticPalette(createEmptyDocument());
      const tree = buildModalShellV1({ title: 'T', theme: 'system' }) as unknown as Frame;
      const cardFill = getFillColor(findByRole(tree, 'modal-shell-card')!)!;
      const resolved = resolveColorRef(cardFill, doc.variables!, { Mode: 'Light' });
      expect(resolved).toBe('#FFFFFF');
    });

    it('Dark mode: $color-surface resolves to #1E293B', () => {
      const doc = applySemanticPalette(createEmptyDocument());
      const tree = buildModalShellV1({ title: 'T', theme: 'system' }) as unknown as Frame;
      const cardFill = getFillColor(findByRole(tree, 'modal-shell-card')!)!;
      const resolved = resolveColorRef(cardFill, doc.variables!, { Mode: 'Dark' });
      expect(resolved).toBe('#1E293B');
    });

    it('Dark mode: every v1 system-theme ref resolves to a concrete dark value', () => {
      const doc = applySemanticPalette(createEmptyDocument());
      const tree = buildModalShellV1({
        title: 'T',
        subtitle: 'S',
        theme: 'system',
      }) as unknown as Frame;
      const card = findByRole(tree, 'modal-shell-card')!;
      const title = findByRole(tree, 'modal-title')!;
      const sub = findByRole(tree, 'modal-subtitle')!;

      expect(resolveColorRef(getFillColor(card)!, doc.variables!, { Mode: 'Dark' })).toBe(
        '#1E293B',
      );
      expect(resolveColorRef(getFillColor(title)!, doc.variables!, { Mode: 'Dark' })).toBe(
        '#F1F5F9',
      );
      expect(resolveColorRef(getFillColor(sub)!, doc.variables!, { Mode: 'Dark' })).toBe('#94A3B8');
    });

    it('WITHOUT palette seeded: refs resolve to DEFAULT_PALETTE_FALLBACK values (P1.6)', () => {
      const doc = createEmptyDocument(); // no palette applied
      const tree = buildModalShellV1({ title: 'T', theme: 'system' }) as unknown as Frame;
      const cardFill = getFillColor(findByRole(tree, 'modal-shell-card')!)!;
      const resolved = resolveColorRef(cardFill, doc.variables ?? {}, { Mode: 'Dark' });
      // P1.6: resolver now falls back to DEFAULT_PALETTE_FALLBACK so that
      // v1 'system' mode on an un-seeded doc is byte-equivalent to v0 hex
      // output (spec §5.3). $color-surface dark = #1E293B.
      expect(resolved).toBe('#1E293B');
    });
  });

  describe('param handling consistent with v0', () => {
    it('card_width clamps to min 280', () => {
      const t = buildModalShellV1({ title: 'T', card_width: 100 }) as unknown as Frame & {
        children: Array<{ width: number }>;
      };
      expect(t.children[0].width).toBe(280);
    });

    it('scrim_opacity=0 → empty scrim fill array (no backdrop)', () => {
      const t = buildModalShellV1({ title: 'T', scrim_opacity: 0 }) as unknown as Frame;
      expect(t.fill).toEqual([]);
    });

    it('subtitle omitted → only title child in card', () => {
      const t = buildModalShellV1({ title: 'Just a title' }) as unknown as Frame;
      const card = findByRole(t, 'modal-shell-card')!;
      expect(card.children!.length).toBe(1);
      expect(card.children![0].role).toBe('modal-title');
    });
  });
});
