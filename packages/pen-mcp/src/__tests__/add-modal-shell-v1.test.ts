import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddModalShellV1 } from '../tools/add-modal-shell-v1';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-modal-shell-v1');
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

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['m.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_modal_shell_v1', () => {
  it('registered; required=[title]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_modal_shell_v1')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_modal_shell_v1');
    expect(def?.inputSchema.required).toEqual(['title']);
  });

  it('schema exposes enum [light, dark, system] for theme param', () => {
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_modal_shell_v1');
    const props = def?.inputSchema.properties as unknown as Record<string, { enum?: unknown }>;
    expect(props.theme?.enum).toEqual(['light', 'dark', 'system']);
  });

  it('theme omitted → card fill #FFFFFF (light default, v0-parity)', async () => {
    const fp = await fresh('m.op');
    await handleAddModalShellV1({ filePath: fp, title: 'Delete?' });
    const root = getRoot(await readDoc(fp));
    const card = findByRole(root, 'modal-shell-card')!;
    expect(fillColor(card)).toBe('#FFFFFF');
  });

  it('theme=dark → card fill #1E293B', async () => {
    const fp = await fresh('m.op');
    await handleAddModalShellV1({ filePath: fp, title: 'Delete?', theme: 'dark' });
    const root = getRoot(await readDoc(fp));
    const card = findByRole(root, 'modal-shell-card')!;
    expect(fillColor(card)).toBe('#1E293B');
  });

  it('theme=dark → subtitle fill #94A3B8', async () => {
    const fp = await fresh('m.op');
    await handleAddModalShellV1({
      filePath: fp,
      title: 'T',
      subtitle: 'Cannot undo.',
      theme: 'dark',
    });
    const root = getRoot(await readDoc(fp));
    const sub = findByRole(root, 'modal-subtitle')!;
    expect(fillColor(sub)).toBe('#94A3B8');
  });

  it('theme=system → card fill emits $color-surface ref', async () => {
    const fp = await fresh('m.op');
    await handleAddModalShellV1({ filePath: fp, title: 'T', theme: 'system' });
    const root = getRoot(await readDoc(fp));
    const card = findByRole(root, 'modal-shell-card')!;
    expect(fillColor(card)).toBe('$color-surface');
  });

  it('theme=system → title fill $color-text-primary, subtitle $color-text-muted', async () => {
    const fp = await fresh('m.op');
    await handleAddModalShellV1({
      filePath: fp,
      title: 'T',
      subtitle: 'S',
      theme: 'system',
    });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(findByRole(root, 'modal-title')!)).toBe('$color-text-primary');
    expect(fillColor(findByRole(root, 'modal-subtitle')!)).toBe('$color-text-muted');
  });

  it('scrim stays #000000 in ALL themes (modal backdrop is never themed)', async () => {
    const fp = await fresh('m.op');
    await handleAddModalShellV1({ filePath: fp, title: 'T', theme: 'dark' });
    const dark = getRoot(await readDoc(fp));
    expect(fillColor(findByRole(dark, 'modal-scrim')!)).toBe('#000000');

    // Also check system
    const fp2 = await fresh('m.op');
    await handleAddModalShellV1({ filePath: fp2, title: 'T', theme: 'system' });
    const sys = getRoot(await readDoc(fp2));
    expect(fillColor(findByRole(sys, 'modal-scrim')!)).toBe('#000000');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('m.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddModalShellV1({
        filePath: fp,
        title: 'T',
        theme: 'dark',
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
