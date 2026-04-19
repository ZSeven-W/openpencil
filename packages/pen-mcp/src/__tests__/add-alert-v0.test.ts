import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddAlertV0 } from '../tools/add-alert-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-alert-v0');
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

describe('add_alert_v0', () => {
  it('registered + required message', () => {
    expect(DESIGN_TOOL_NAMES.has('add_alert_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_alert_v0');
    expect(def?.inputSchema.required).toEqual(['message']);
  });

  it('minimal: message only (1 child)', async () => {
    const fp = await fresh('a.op');
    await handleAddAlertV0({ filePath: fp, message: 'Heads up.' });
    const a = getRoot(await readDoc(fp));
    expect(a.role).toBe('alert');
    expect(a.width).toBe('fill_container');
    expect(a.cornerRadius).toBe(8);
    expect(a.padding).toEqual([12, 16]);
    const kids = a.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].role).toBe('alert-message');
    expect(kids[0].content).toBe('Heads up.');
  });

  it('icon + dismissible: 3 children', async () => {
    const fp = await fresh('a.op');
    await handleAddAlertV0({
      filePath: fp,
      message: 'Saved.',
      icon: 'check',
      dismissible: true,
    });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(3);
    expect(kids[0].iconFontName).toBe('check');
    expect(kids[1].role).toBe('alert-message');
    expect(kids[2].role).toBe('alert-close');
    expect(kids[2].iconFontName).toBe('x');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddAlertV0({ filePath: fp, message: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
