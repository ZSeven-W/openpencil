import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddQuoteBlockV0 } from '../tools/add-quote-block-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-quote-block-v0');
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

describe('add_quote_block_v0', () => {
  it('registered + required quote', () => {
    expect(DESIGN_TOOL_NAMES.has('add_quote_block_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_quote_block_v0');
    expect(def?.inputSchema.required).toEqual(['quote']);
  });

  it('with author: 2 text children, author prefixed with em dash', async () => {
    const fp = await fresh('a.op');
    await handleAddQuoteBlockV0({
      filePath: fp,
      quote: 'The only way to do great work is to love what you do.',
      author: 'Steve Jobs',
    });
    const block = getRoot(await readDoc(fp));
    expect(block.role).toBe('quote-block');
    expect(block.width).toBe('fill_container');
    expect(block.layout).toBe('vertical');
    const kids = block.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[0].role).toBe('quote-text');
    expect(kids[0].width).toBe('fill_container');
    expect(kids[0].textGrowth).toBe('fixed-width');
    expect(kids[1].role).toBe('quote-author');
    expect(kids[1].content).toBe('— Steve Jobs');
  });

  it('without author: single text child', async () => {
    const fp = await fresh('a.op');
    await handleAddQuoteBlockV0({ filePath: fp, quote: 'Short quote' });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('quote-text');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddQuoteBlockV0({ filePath: fp, quote: 'x', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
