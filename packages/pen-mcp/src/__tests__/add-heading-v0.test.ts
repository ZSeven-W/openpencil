import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddHeadingV0 } from '../tools/add-heading-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-heading-v0');
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

describe('add_heading_v0', () => {
  it('registered + required content; level is enum with h2 default', () => {
    expect(DESIGN_TOOL_NAMES.has('add_heading_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_heading_v0');
    expect(def?.inputSchema.required).toEqual(['content']);
    const levelProp = (def?.inputSchema.properties as any)?.level;
    expect(levelProp?.enum).toEqual(['display', 'h1', 'h2', 'h3']);
  });

  it('default h2: 24/600/1.2 — lineHeight enforced (the core invariant)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'Welcome' });
    const h = getRoot(await readDoc(fp));
    expect(h.type).toBe('text');
    expect(h.role).toBe('heading');
    expect(h.content).toBe('Welcome');
    expect(h.fontSize).toBe(24);
    expect(h.fontWeight).toBe(600);
    expect(h.lineHeight).toBe(1.2); // NOT 1.5 (default) — which stacks tight
  });

  it('display: 48/700/1.0 + letterSpacing=-0.5', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'Hero', level: 'display' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontSize).toBe(48);
    expect(h.fontWeight).toBe(700);
    expect(h.lineHeight).toBe(1.0);
    expect(h.letterSpacing).toBe(-0.5);
  });

  it('h1: 32/700/1.1 (no letterSpacing set)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'H1 Title', level: 'h1' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontSize).toBe(32);
    expect(h.fontWeight).toBe(700);
    expect(h.lineHeight).toBe(1.1);
    expect(h.letterSpacing).toBeUndefined();
  });

  it('h3: 20/600/1.25 (card / list headers)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'Card Title', level: 'h3' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontSize).toBe(20);
    expect(h.fontWeight).toBe(600);
    expect(h.lineHeight).toBe(1.25);
  });

  it('has a valid id', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'X' });
    expect(typeof getRoot(await readDoc(fp)).id).toBe('string');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddHeadingV0({ filePath: fp, content: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});

describe('add_heading_v0 — CJK typography (regression for Codex #13)', () => {
  it('Chinese display: Noto Sans SC + lineHeight 1.3 + NO negative letterSpacing', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: '你好世界', level: 'display' });
    const h = getRoot(await readDoc(fp));
    // Memory: "NEVER use Space Grotesk/Manrope for CJK — no CJK glyphs"
    expect(h.fontFamily).toBe('Noto Sans SC');
    // Memory: "CJK headings 1.3-1.4 (NOT 1.1-1.2 like Latin)" —
    // Latin display is 1.0, CJK display bumps to 1.3
    expect(h.lineHeight).toBe(1.3);
    // Memory: "CJK letterSpacing: 0, NEVER negative" — Latin display
    // has letterSpacing -0.5, CJK display drops it entirely
    expect(h.letterSpacing).toBeUndefined();
    // Structural parity: fontSize / fontWeight unchanged
    expect(h.fontSize).toBe(48);
    expect(h.fontWeight).toBe(700);
  });

  it('Chinese h1 / h2 / h3: lineHeight 1.3 / 1.35 / 1.4, Noto Sans SC', async () => {
    // Use unique filenames per iteration — reusing one path confuses
    // openDocument's cache across iterations of the same test run.
    const cases: Array<{ level: 'h1' | 'h2' | 'h3'; lh: number; file: string }> = [
      { level: 'h1', lh: 1.3, file: 'cjk-h1.op' },
      { level: 'h2', lh: 1.35, file: 'cjk-h2.op' },
      { level: 'h3', lh: 1.4, file: 'cjk-h3.op' },
    ];
    try {
      for (const { level, lh, file } of cases) {
        const fp = join(TMP, file);
        await writeFile(fp, EMPTY, 'utf-8');
        await handleAddHeadingV0({ filePath: fp, content: '中文标题', level });
        const h = getRoot(await readDoc(fp));
        expect(h.fontFamily, `${level} fontFamily`).toBe('Noto Sans SC');
        expect(h.lineHeight, `${level} lineHeight`).toBe(lh);
        // CJK never has letterSpacing (and never negative)
        expect(h.letterSpacing, `${level} letterSpacing`).toBeUndefined();
      }
    } finally {
      for (const { file } of cases) {
        try {
          const fp = join(TMP, file);
          invalidateCache(fp);
          await unlink(fp);
        } catch {}
      }
    }
  });

  it('Japanese content gets Noto Sans JP (NOT SC — respects script-specific font contract)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'こんにちは' });
    const h = getRoot(await readDoc(fp));
    // text-rules.md: "heading='Noto Sans JP' (Japanese)"
    expect(h.fontFamily).toBe('Noto Sans JP');
    expect(h.lineHeight).toBe(1.35); // h2 CJK
  });

  it('Korean content gets Noto Sans KR (script-specific, not SC fallback)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: '안녕하세요' });
    const h = getRoot(await readDoc(fp));
    // text-rules.md: "heading='Noto Sans KR' (Korean)"
    expect(h.fontFamily).toBe('Noto Sans KR');
    expect(h.lineHeight).toBe(1.35);
  });

  it('Chinese content gets Noto Sans SC', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: '你好世界' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontFamily).toBe('Noto Sans SC');
  });

  it('Japanese with Han ideographs (kanji+hiragana) still detects Japanese', async () => {
    // "今日は" (konnichiwa) — Chinese-looking kanji but hiragana
    // marker makes it Japanese
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: '今日は' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontFamily).toBe('Noto Sans JP');
  });

  it('mixed Latin + Chinese takes Chinese preset (any Han ideograph wins)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'Hello 世界' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontFamily).toBe('Noto Sans SC');
    expect(h.lineHeight).toBe(1.35);
  });

  it('pure Latin content keeps Latin preset (no fontFamily, original lineHeight)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV0({ filePath: fp, content: 'Hello World', level: 'h1' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontFamily).toBeUndefined(); // Latin inherits theme default
    expect(h.lineHeight).toBe(1.1);
  });
});
