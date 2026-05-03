import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddModalShellV0 } from '../tools/add-modal-shell-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-modal-shell-v0');
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

describe('add_modal_shell_v0', () => {
  it('registered; required title', () => {
    expect(DESIGN_TOOL_NAMES.has('add_modal_shell_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_modal_shell_v0');
    expect(def?.inputSchema.required).toEqual(['title']);
  });

  it('minimal: scrim (50% black) + centered 400px card + title', async () => {
    const fp = await fresh('a.op');
    await handleAddModalShellV0({ filePath: fp, title: 'Delete?' });
    const scrim = getRoot(await readDoc(fp));
    expect(scrim.role).toBe('modal-scrim');
    expect(scrim.width).toBe('fill_container');
    expect(scrim.height).toBe('fill_container');
    expect(scrim.alignItems).toBe('center');
    expect(scrim.justifyContent).toBe('center');
    // Scrim fill @ 0.5 opacity
    const scrimFill = scrim.fill as Array<{ color: string; opacity: number }>;
    expect(scrimFill.length).toBe(1);
    expect(scrimFill[0].color).toBe('#000000');
    expect(scrimFill[0].opacity).toBe(0.5);

    const card = (scrim.children as Record<string, unknown>[])[0];
    expect(card.role).toBe('modal-shell-card');
    expect(card.width).toBe(400);
    expect(card.cornerRadius).toBe(16);
    // Shadow effect
    const effects = card.effects as Array<{ type: string }>;
    expect(effects[0].type).toBe('shadow');

    const cardKids = card.children as Record<string, unknown>[];
    expect(cardKids.length).toBe(1); // title only
    expect(cardKids[0].role).toBe('modal-title');
    expect(cardKids[0].content).toBe('Delete?');
  });

  it('subtitle adds a second text below the title', async () => {
    const fp = await fresh('a.op');
    await handleAddModalShellV0({
      filePath: fp,
      title: 'Confirm',
      subtitle: 'This cannot be undone.',
    });
    const card = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    const cardKids = card.children as Record<string, unknown>[];
    expect(cardKids.length).toBe(2);
    expect(cardKids[1].role).toBe('modal-subtitle');
    expect(cardKids[1].content).toBe('This cannot be undone.');
  });

  it('scrim_opacity=0 → empty fill array (borderless no-scrim variant)', async () => {
    const fp = await fresh('a.op');
    await handleAddModalShellV0({ filePath: fp, title: 'X', scrim_opacity: 0 });
    const scrim = getRoot(await readDoc(fp));
    const scrimFill = scrim.fill as unknown[];
    expect(scrimFill.length).toBe(0);
  });

  it('card_width honored (clamped to >=280)', async () => {
    const fp = await fresh('a.op');
    await handleAddModalShellV0({ filePath: fp, title: 'X', card_width: 100 });
    const card = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(card.width).toBe(280);
  });

  it('card_padding honored (clamped to >=12)', async () => {
    const fp = await fresh('a.op');
    await handleAddModalShellV0({ filePath: fp, title: 'X', card_padding: 5 });
    const card = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(card.padding).toBe(12);
  });

  it('modal-shell-card role is the documented insertion target', async () => {
    // The tool's key contract is that body content goes into the role
    // 'modal-shell-card'. Regression check that role stays named.
    const fp = await fresh('a.op');
    await handleAddModalShellV0({ filePath: fp, title: 'X' });
    const card = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(card.role).toBe('modal-shell-card');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddModalShellV0({ filePath: fp, title: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
