import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddBodyTextV0 } from '../tools/add-body-text-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-body-text-v0');
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

describe('add_body_text_v0', () => {
  it('registered + required content', () => {
    expect(DESIGN_TOOL_NAMES.has('add_body_text_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_body_text_v0');
    expect(def?.inputSchema.required).toEqual(['content']);
  });

  it('Latin content: Inter + lineHeight 1.5 + no letterSpacing override', async () => {
    const fp = await fresh('a.op');
    await handleAddBodyTextV0({
      filePath: fp,
      content: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit.',
    });
    const t = getRoot(await readDoc(fp));
    expect(t.type).toBe('text');
    expect(t.role).toBe('body');
    expect(t.fontFamily).toBe('Inter');
    expect(t.lineHeight).toBe(1.5);
    expect(t.letterSpacing).toBeUndefined(); // NOT set for Latin
    expect(t.fontSize).toBe(16);
    expect(t.fontWeight).toBe(400);
    // Wrap-safe: fill_container + fixed-width in vertical parent
    expect(t.width).toBe('fill_container');
    expect(t.textGrowth).toBe('fixed-width');
  });

  it('Chinese content: Noto Sans SC + lineHeight 1.6 + letterSpacing 0', async () => {
    const fp = await fresh('a.op');
    await handleAddBodyTextV0({
      filePath: fp,
      content: '你好世界，这是一段中文正文测试。',
    });
    const t = getRoot(await readDoc(fp));
    // text-rules.md: body Chinese → Noto Sans SC
    expect(t.fontFamily).toBe('Noto Sans SC');
    expect(t.lineHeight).toBe(1.6);
    expect(t.letterSpacing).toBe(0);
  });

  it('Japanese content: Noto Sans JP (NOT SC — respects script-specific font contract)', async () => {
    const fp = await fresh('a.op');
    await handleAddBodyTextV0({ filePath: fp, content: 'こんにちは、テキストです。' });
    const t = getRoot(await readDoc(fp));
    // text-rules.md: "body='Noto Sans JP' (Japanese)"
    expect(t.fontFamily).toBe('Noto Sans JP');
    expect(t.lineHeight).toBe(1.6);
    expect(t.letterSpacing).toBe(0);
  });

  it('Korean content: Noto Sans KR (script-specific, not SC fallback)', async () => {
    const fp = await fresh('a.op');
    await handleAddBodyTextV0({ filePath: fp, content: '안녕하세요, 본문입니다.' });
    const t = getRoot(await readDoc(fp));
    // text-rules.md: "body='Noto Sans KR' (Korean)"
    expect(t.fontFamily).toBe('Noto Sans KR');
    expect(t.lineHeight).toBe(1.6);
  });

  it('Japanese with Han ideographs (kanji+hiragana) still detects Japanese', async () => {
    const fp = await fresh('a.op');
    await handleAddBodyTextV0({ filePath: fp, content: '今日は良い天気です。' });
    const t = getRoot(await readDoc(fp));
    expect(t.fontFamily).toBe('Noto Sans JP');
  });

  it('mixed Chinese + Latin content triggers Chinese (any Han char wins)', async () => {
    const fp = await fresh('a.op');
    await handleAddBodyTextV0({ filePath: fp, content: 'Hello 你好' });
    const t = getRoot(await readDoc(fp));
    expect(t.fontFamily).toBe('Noto Sans SC');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddBodyTextV0({ filePath: fp, content: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
