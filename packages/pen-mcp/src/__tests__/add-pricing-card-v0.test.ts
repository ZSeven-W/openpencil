import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddPricingCardV0 } from '../tools/add-pricing-card-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-pricing-card-v0');
const EMPTY = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP, name);
  await writeFile(fp, EMPTY, 'utf-8');
  return fp;
}
async function readDoc(fp: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(fp, 'utf-8'));
}
function getRoot(doc: Record<string, unknown>): Record<string, unknown> {
  const pages = doc['pages'] as Array<{ children?: Record<string, unknown>[] }> | undefined;
  const top = doc['children'] as Record<string, unknown>[] | undefined;
  const root = (top ?? pages?.[0]?.children)?.[0];
  if (!root) throw new Error('no root');
  return root;
}
function findAllByRole(n: Record<string, unknown>, role: string): Record<string, unknown>[] {
  const out: Record<string, unknown>[] = [];
  if (n.role === role) out.push(n);
  const kids = (n.children ?? []) as Record<string, unknown>[];
  for (const c of kids) out.push(...findAllByRole(c, role));
  return out;
}
function findByRole(n: Record<string, unknown>, role: string): Record<string, unknown> | undefined {
  return findAllByRole(n, role)[0];
}
function fillColor(n: Record<string, unknown> | undefined): string | undefined {
  const fills = n?.fill as Array<{ color?: string }> | undefined;
  return fills?.[0]?.color;
}
function strokeColor(n: Record<string, unknown>): string | undefined {
  const stroke = n.stroke as { fill?: Array<{ color?: string }> } | undefined;
  return stroke?.fill?.[0]?.color;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['p.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_pricing_card_v0', () => {
  it('registered; required=[tier, price]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_pricing_card_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_pricing_card_v0');
    expect(def?.inputSchema.required).toEqual(['tier', 'price']);
  });

  it('minimal: tier + price → default emphasis, no features, slate CTA, $ prefix', async () => {
    const fp = await fresh('p.op');
    await handleAddPricingCardV0({ filePath: fp, tier: 'Starter', price: '9' });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('pricing-card');
    expect(strokeColor(root)).toBe('#E2E8F0');
    expect(findByRole(root, 'pricing-tier')!.content).toBe('Starter');
    expect(findByRole(root, 'pricing-currency')!.content).toBe('$');
    expect(findByRole(root, 'pricing-amount')!.content).toBe('9');
    expect(findByRole(root, 'pricing-period')).toBeUndefined(); // no period given
    expect(findAllByRole(root, 'pricing-feature')).toHaveLength(0);
    expect(fillColor(findByRole(root, 'pricing-cta'))).toBe('#0F172A');
    expect(findByRole(root, 'pricing-cta-label')!.content).toBe('Get started');
    expect(findByRole(root, 'pricing-badge')).toBeUndefined();
  });

  it('featured emphasis → accent border + accent CTA + auto "Most popular" badge', async () => {
    const fp = await fresh('p.op');
    await handleAddPricingCardV0({
      filePath: fp,
      tier: 'Pro',
      price: '29',
      period: '/month',
      emphasis: 'featured',
    });
    const root = getRoot(await readDoc(fp));
    expect(strokeColor(root)).toBe('#2563EB');
    expect(fillColor(findByRole(root, 'pricing-cta'))).toBe('#2563EB');
    expect(findByRole(root, 'pricing-badge-label')!.content).toBe('Most popular');
  });

  it('explicit badge overrides the auto label on featured', async () => {
    const fp = await fresh('p.op');
    await handleAddPricingCardV0({
      filePath: fp,
      tier: 'Enterprise',
      price: 'Custom',
      emphasis: 'featured',
      badge: 'Recommended',
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'pricing-badge-label')!.content).toBe('Recommended');
  });

  it('features list renders with lucide check icon + label per row', async () => {
    const fp = await fresh('p.op');
    await handleAddPricingCardV0({
      filePath: fp,
      tier: 'Team',
      price: '99',
      features: ['10 seats included', 'Priority support', 'Advanced analytics'],
    });
    const root = getRoot(await readDoc(fp));
    const feats = findAllByRole(root, 'pricing-feature');
    expect(feats).toHaveLength(3);
    expect(findAllByRole(root, 'pricing-feature-check')).toHaveLength(3);
    const labels = findAllByRole(root, 'pricing-feature-label').map((f) => f.content);
    expect(labels).toEqual(['10 seats included', 'Priority support', 'Advanced analytics']);
  });

  it('featured feature-check uses accent color; default uses emerald', async () => {
    const fpA = await fresh('p.op');
    await handleAddPricingCardV0({
      filePath: fpA,
      tier: 'Pro',
      price: '29',
      features: ['A'],
      emphasis: 'featured',
    });
    expect(fillColor(findByRole(getRoot(await readDoc(fpA)), 'pricing-feature-check'))).toBe(
      '#2563EB',
    );
    invalidateCache(fpA);
    await writeFile(fpA, EMPTY, 'utf-8');

    await handleAddPricingCardV0({ filePath: fpA, tier: 'Starter', price: '0', features: ['B'] });
    expect(fillColor(findByRole(getRoot(await readDoc(fpA)), 'pricing-feature-check'))).toBe(
      '#10B981',
    );
  });

  it('description rendered in header when provided', async () => {
    const fp = await fresh('p.op');
    await handleAddPricingCardV0({
      filePath: fp,
      tier: 'Team',
      price: '49',
      description: 'For growing teams',
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'pricing-description')!.content).toBe('For growing teams');
  });

  it('currency + period overrides work (¥/月 / seat)', async () => {
    const fp = await fresh('p.op');
    await handleAddPricingCardV0({
      filePath: fp,
      tier: 'Pro',
      price: '99',
      currency: '¥',
      period: '/月',
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'pricing-currency')!.content).toBe('¥');
    expect(findByRole(root, 'pricing-period')!.content).toBe('/月');
  });

  it('width clamps (< 220 → 220)', async () => {
    const fp = await fresh('p.op');
    await handleAddPricingCardV0({ filePath: fp, tier: 'S', price: '0', width: 100 });
    const root = getRoot(await readDoc(fp));
    expect(root.width).toBe(220);
  });

  it('clamps features to 12 max', async () => {
    const fp = await fresh('p.op');
    const many = Array.from({ length: 20 }, (_, i) => `Feature ${i}`);
    await handleAddPricingCardV0({
      filePath: fp,
      tier: 'Enterprise',
      price: 'Custom',
      features: many,
    });
    const root = getRoot(await readDoc(fp));
    expect(findAllByRole(root, 'pricing-feature')).toHaveLength(12);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('p.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddPricingCardV0({
        filePath: fp,
        tier: 'Pro',
        price: '29',
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
