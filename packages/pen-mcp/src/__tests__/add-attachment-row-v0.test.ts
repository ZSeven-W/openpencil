import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddAttachmentRowV0 } from '../tools/add-attachment-row-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-attachment-row-v0');
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

describe('add_attachment_row_v0', () => {
  it('registered; required=[filename]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_attachment_row_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_attachment_row_v0');
    expect(def?.inputSchema.required).toEqual(['filename']);
  });

  it('minimal: filename only → icon + filename + remove × (no size)', async () => {
    const fp = await fresh('a.op');
    await handleAddAttachmentRowV0({ filePath: fp, filename: 'notes.txt' });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('attachment-row');
    // Icon present + meta + remove
    expect(findByRole(root, 'attachment-icon')).toBeDefined();
    expect(findByRole(root, 'attachment-filename')!.content).toBe('notes.txt');
    expect(findByRole(root, 'attachment-size')).toBeUndefined();
    expect(findByRole(root, 'attachment-remove')).toBeDefined();
  });

  it('with size → muted text below filename', async () => {
    const fp = await fresh('a.op');
    await handleAddAttachmentRowV0({
      filePath: fp,
      filename: 'report.pdf',
      size: '1.2 MB',
    });
    const root = getRoot(await readDoc(fp));
    const sizeNode = findByRole(root, 'attachment-size')!;
    expect(sizeNode).toBeDefined();
    expect(sizeNode.content).toBe('1.2 MB');
  });

  it('custom icon shows up verbatim on the type-icon node', async () => {
    const fp = await fresh('a.op');
    await handleAddAttachmentRowV0({
      filePath: fp,
      filename: 'pic.jpg',
      icon: 'file-image',
    });
    const root = getRoot(await readDoc(fp));
    const icon = findByRole(root, 'attachment-icon')!;
    expect(icon.iconFontName).toBe('file-image');
  });

  it('removable=false drops the × icon', async () => {
    const fp = await fresh('a.op');
    await handleAddAttachmentRowV0({
      filePath: fp,
      filename: 'sealed.zip',
      removable: false,
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'attachment-remove')).toBeUndefined();
  });

  it('default icon is "file"', async () => {
    const fp = await fresh('a.op');
    await handleAddAttachmentRowV0({ filePath: fp, filename: 'x' });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'attachment-icon')!.iconFontName).toBe('file');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddAttachmentRowV0({ filePath: fp, filename: 'x', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
