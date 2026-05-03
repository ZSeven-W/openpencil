import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddFaqItemV0 } from '../tools/add-faq-item-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-faq-item-v0');
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

describe('add_faq_item_v0', () => {
  it('registered; required=[question]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_faq_item_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_faq_item_v0');
    expect(def?.inputSchema.required).toEqual(['question']);
  });

  it('collapsed (default): header only, chevron-right, no answer', async () => {
    const fp = await fresh('a.op');
    await handleAddFaqItemV0({ filePath: fp, question: 'Is it free?' });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('faq-item');
    const kids = root.children as Record<string, unknown>[];
    // header only — no answer
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('faq-header');
    const header = kids[0].children as Record<string, unknown>[];
    expect(header[0].content).toBe('Is it free?');
    expect(header[1].iconFontName).toBe('chevron-right');
    expect(header[1].role).toBe('faq-chevron-closed');
  });

  it('expanded: header + answer, chevron-down', async () => {
    const fp = await fresh('a.op');
    await handleAddFaqItemV0({
      filePath: fp,
      question: 'How do I cancel?',
      answer: 'Email support@example.com.',
      expanded: true,
    });
    const root = getRoot(await readDoc(fp));
    const kids = root.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    const header = kids[0].children as Record<string, unknown>[];
    expect(header[1].iconFontName).toBe('chevron-down');
    expect(header[1].role).toBe('faq-chevron-open');
    expect(kids[1].role).toBe('faq-answer');
    expect(kids[1].content).toBe('Email support@example.com.');
  });

  it('expanded without answer → still no answer emitted (guards against undefined)', async () => {
    const fp = await fresh('a.op');
    await handleAddFaqItemV0({ filePath: fp, question: 'Q', expanded: true });
    const root = getRoot(await readDoc(fp));
    const kids = root.children as Record<string, unknown>[];
    expect(kids.length).toBe(1); // no answer child
  });

  it('show_divider=true appends 1px hairline', async () => {
    const fp = await fresh('a.op');
    await handleAddFaqItemV0({
      filePath: fp,
      question: 'Q',
      show_divider: true,
    });
    const root = getRoot(await readDoc(fp));
    const kids = root.children as Record<string, unknown>[];
    const divider = kids.find((k) => k.role === 'faq-divider');
    expect(divider).toBeDefined();
    expect(divider!.type).toBe('rectangle');
    expect(divider!.height).toBe(1);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddFaqItemV0({ filePath: fp, question: 'Q', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
