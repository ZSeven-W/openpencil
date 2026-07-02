import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddCodeBlockV0 } from '../tools/add-code-block-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-code-block-v0');
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

describe('add_code_block_v0', () => {
  it('registered + required code', () => {
    expect(DESIGN_TOOL_NAMES.has('add_code_block_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_code_block_v0');
    expect(def?.inputSchema.required).toEqual(['code']);
  });

  it('preserves newlines in code content', async () => {
    const fp = await fresh('a.op');
    const code = 'const x = 1;\nconst y = 2;\nconsole.log(x + y);';
    await handleAddCodeBlockV0({ filePath: fp, code });
    const block = getRoot(await readDoc(fp));
    expect(block.role).toBe('code-block');
    expect(block.width).toBe('fill_container');
    const text = (block.children as Record<string, unknown>[])[0];
    expect(text.role).toBe('code');
    expect(text.content).toBe(code);
    expect(text.textGrowth).toBe('fixed-width');
    expect(text.width).toBe('fill_container');
  });

  it('language shows in frame name', async () => {
    const fp = await fresh('a.op');
    await handleAddCodeBlockV0({ filePath: fp, code: 'x', language: 'typescript' });
    const block = getRoot(await readDoc(fp));
    expect(block.name).toBe('Code Block (typescript)');
  });

  it('no language: plain name', async () => {
    const fp = await fresh('a.op');
    await handleAddCodeBlockV0({ filePath: fp, code: 'x' });
    const block = getRoot(await readDoc(fp));
    expect(block.name).toBe('Code Block');
  });
});
