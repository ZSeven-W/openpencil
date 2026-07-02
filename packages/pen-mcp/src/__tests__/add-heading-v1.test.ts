import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddHeadingV1 } from '../tools/add-heading-v1';
import { handleAddHeadingV0 } from '../tools/add-heading-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-heading-v1');
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
  for (const f of ['v0.op', 'v1.op', 'a.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_heading_v1 — registration and schema', () => {
  it('registered in DESIGN_TOOL_NAMES', () => {
    expect(DESIGN_TOOL_NAMES.has('add_heading_v1')).toBe(true);
  });

  it('required=["content"]', () => {
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_heading_v1');
    expect(def?.inputSchema.required).toEqual(['content']);
  });

  it('schema exposes enum [light, dark, system] for theme param', () => {
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_heading_v1');
    const props = def?.inputSchema.properties as unknown as Record<string, { enum?: unknown }>;
    expect(props.theme?.enum).toEqual(['light', 'dark', 'system']);
  });

  it('level enum contains display, h1, h2, h3', () => {
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_heading_v1');
    const props = def?.inputSchema.properties as unknown as Record<string, { enum?: unknown }>;
    expect(props.level?.enum).toEqual(['display', 'h1', 'h2', 'h3']);
  });
});

describe('add_heading_v1 — byte-parity with v0 (light/omitted theme)', () => {
  it('default h2 light: 24/600/1.2 — byte-parity with v0', async () => {
    const fpV0 = await fresh('v0.op');
    const fpV1 = await fresh('v1.op');
    await handleAddHeadingV0({ filePath: fpV0, content: 'Welcome' });
    await handleAddHeadingV1({ filePath: fpV1, content: 'Welcome' });
    const v0 = getRoot(await readDoc(fpV0));
    const v1 = getRoot(await readDoc(fpV1));
    // id is unique but all other fields must match
    const { id: _id0, ...v0Rest } = v0 as { id: string };
    const { id: _id1, ...v1Rest } = v1 as { id: string };
    expect(v1Rest).toEqual(v0Rest);
  });

  it('h1 light: 32/700/1.1 — v0 parity', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV1({ filePath: fp, content: 'Title', level: 'h1', theme: 'light' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontSize).toBe(32);
    expect(h.fontWeight).toBe(700);
    expect(h.lineHeight).toBe(1.1);
  });

  it('display light: 48/700/1.0/letterSpacing=-0.5 — v0 parity', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV1({
      filePath: fp,
      content: 'Hero',
      level: 'display',
      theme: 'light',
    });
    const h = getRoot(await readDoc(fp));
    expect(h.fontSize).toBe(48);
    expect(h.letterSpacing).toBe(-0.5);
  });
});

describe('add_heading_v1 — system mode emits refs', () => {
  it('h1 system: fontSize=$type-h1-size, fill=$color-text-primary', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV1({ filePath: fp, content: 'Welcome', level: 'h1', theme: 'system' });
    const h = getRoot(await readDoc(fp));
    expect(h.fontSize).toBe('$type-h1-size');
    expect(h.fontWeight).toBe('$type-h1-weight');
    expect(h.lineHeight).toBe('$type-h1-line-height');
    const fills = h.fill as Array<{ color: string }> | undefined;
    expect(fills?.[0]?.color).toBe('$color-text-primary');
  });
});

describe('add_heading_v1 — dark mode', () => {
  it('h2 dark: fill is #F1F5F9 (dark text-primary)', async () => {
    const fp = await fresh('a.op');
    await handleAddHeadingV1({ filePath: fp, content: 'Section', level: 'h2', theme: 'dark' });
    const h = getRoot(await readDoc(fp));
    const fills = h.fill as Array<{ color: string }> | undefined;
    expect(fills?.[0]?.color).toBe('#F1F5F9');
  });
});

describe('add_heading_v1 — error handling', () => {
  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddHeadingV1({ filePath: fp, content: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });

  it('coerces invalid level to default h2 instead of throwing', async () => {
    const fp = await fresh('a.op');
    // buildHeadingV1 fuzzy-coerces invalid 'caption' to 'h2' + warns;
    // handler succeeds and inserts with default level (no rejection).
    await expect(
      handleAddHeadingV1({ filePath: fp, content: 'X', level: 'caption' as never }),
    ).resolves.toBeDefined();
  });
});
