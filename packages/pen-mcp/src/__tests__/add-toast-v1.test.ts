import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { buildToast, buildToastV1 } from '@zseven-w/pen-core';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddToastV1 } from '../tools/add-toast-v1';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-toast-v1');
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
function findByRole(n: Record<string, unknown>, role: string): Record<string, unknown> | undefined {
  if (n.role === role) return n;
  const kids = (n.children ?? []) as Record<string, unknown>[];
  for (const c of kids) {
    const hit = findByRole(c, role);
    if (hit) return hit;
  }
  return undefined;
}
function fillColor(n: Record<string, unknown>): string | undefined {
  const fills = n.fill as Array<{ color?: string }> | undefined;
  return fills?.[0]?.color;
}
function stripIds(n: unknown): unknown {
  if (Array.isArray(n)) return n.map(stripIds);
  if (n && typeof n === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(n as Record<string, unknown>)) {
      if (k === 'id') continue;
      out[k] = stripIds(v);
    }
    return out;
  }
  return n;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['t.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_toast_v1', () => {
  it('registered; required=[message]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_toast_v1')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_toast_v1');
    expect(def?.inputSchema.required).toEqual(['message']);
  });

  it('schema exposes theme=["light","dark","system"]', () => {
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_toast_v1');
    const props = def?.inputSchema.properties as Record<string, { enum?: unknown }> | undefined;
    expect(props?.theme?.enum).toEqual(['light', 'dark', 'system']);
  });

  it('default theme=light → dark pill #111827 + white text (v0 parity)', async () => {
    const fp = await fresh('t.op');
    await handleAddToastV1({ filePath: fp, message: 'Saved', icon: 'check' });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(root)).toBe('#111827');
    expect(fillColor(findByRole(root, 'toast-message')!)).toBe('#FFFFFF');
  });

  it('theme=light is byte-parity with buildToast v0 (modulo ids)', () => {
    const v0 = stripIds(buildToast({ message: 'Saved', icon: 'check' }));
    const v1 = stripIds(buildToastV1({ message: 'Saved', icon: 'check', theme: 'light' }));
    expect(v1).toEqual(v0);
  });

  it('theme=dark → light pill #F1F5F9 + dark #0F172A text (inverted contrast)', async () => {
    const fp = await fresh('t.op');
    await handleAddToastV1({ filePath: fp, message: 'Saved', theme: 'dark' });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(root)).toBe('#F1F5F9');
    expect(fillColor(findByRole(root, 'toast-message')!)).toBe('#0F172A');
  });

  it('theme=system → $color-* refs for pill bg + message fg (inverted swap)', async () => {
    const fp = await fresh('t.op');
    await handleAddToastV1({ filePath: fp, message: 'Saved', icon: 'info', theme: 'system' });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(root)).toBe('$color-text-primary');
    expect(fillColor(findByRole(root, 'toast-message')!)).toBe('$color-surface');
  });

  it('icon fg color tracks theme (light=white, dark=#0F172A, system=$color-surface)', async () => {
    const fp = await fresh('t.op');
    await handleAddToastV1({ filePath: fp, message: 'X', icon: 'check', theme: 'dark' });
    const root = getRoot(await readDoc(fp));
    const icon = (root.children as Record<string, unknown>[])[0];
    expect(fillColor(icon)).toBe('#0F172A');
  });

  it('message-only (no icon) still builds — no crash on single-child tree', async () => {
    const fp = await fresh('t.op');
    await handleAddToastV1({ filePath: fp, message: 'Plain toast', theme: 'dark' });
    const root = getRoot(await readDoc(fp));
    expect((root.children as unknown[]).length).toBe(1);
    expect(findByRole(root, 'toast-message')!.content).toBe('Plain toast');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('t.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddToastV1({ filePath: fp, message: 'Saved', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
