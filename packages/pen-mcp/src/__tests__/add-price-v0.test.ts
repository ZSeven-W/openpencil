import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddPriceV0 } from '../tools/add-price-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-price-v0');
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

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['a.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_price_v0', () => {
  it('registered + required amount', () => {
    expect(DESIGN_TOOL_NAMES.has('add_price_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_price_v0');
    expect(def?.inputSchema.required).toEqual(['amount']);
  });

  it('$29/month: 3 text parts with right sizes', async () => {
    const fp = await fresh('a.op');
    await handleAddPriceV0({ filePath: fp, amount: '29', period: '/month' });
    const price = getRoot(await readDoc(fp));
    expect(price.role).toBe('price');
    expect(price.alignItems).toBe('flex-end');
    const kids = price.children as Record<string, unknown>[];
    expect(kids.length).toBe(3);
    expect(kids[0].role).toBe('price-currency');
    expect(kids[0].content).toBe('$');
    expect(kids[0].fontSize).toBe(20);
    expect(kids[1].role).toBe('price-amount');
    expect(kids[1].content).toBe('29');
    expect(kids[1].fontSize).toBe(40);
    expect(kids[1].fontWeight).toBe(700);
    expect(kids[2].role).toBe('price-period');
    expect(kids[2].content).toBe('/month');
    expect(kids[2].fontSize).toBe(14);
  });

  it('omits period when not provided: 2 parts', async () => {
    const fp = await fresh('a.op');
    await handleAddPriceV0({ filePath: fp, amount: '99.99' });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
  });

  it('respects custom currency glyph', async () => {
    const fp = await fresh('a.op');
    await handleAddPriceV0({ filePath: fp, amount: '1299', currency: '¥' });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids[0].content).toBe('¥');
  });

  it('amount preserves formatting (thousands + decimals)', async () => {
    const fp = await fresh('a.op');
    await handleAddPriceV0({ filePath: fp, amount: '1,299.99' });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids[1].content).toBe('1,299.99');
  });
});
