import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddImagePlaceholderV0 } from '../tools/add-image-placeholder-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-image-placeholder-v0');
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

describe('add_image_placeholder_v0', () => {
  it('registered; no required params', () => {
    expect(DESIGN_TOOL_NAMES.has('add_image_placeholder_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_image_placeholder_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('default: 200×140 frame + centered image icon (no label)', async () => {
    const fp = await fresh('a.op');
    await handleAddImagePlaceholderV0({ filePath: fp });
    const ph = getRoot(await readDoc(fp));
    expect(ph.role).toBe('image-placeholder');
    expect(ph.type).toBe('frame');
    expect(ph.width).toBe(200);
    expect(ph.height).toBe(140);
    expect(ph.cornerRadius).toBe(8);
    expect(ph.layout).toBe('vertical');
    expect(ph.alignItems).toBe('center');
    expect(ph.justifyContent).toBe('center');
    const kids = ph.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].type).toBe('icon_font');
    expect(kids[0].iconFontName).toBe('image');
  });

  it('label adds a second text child', async () => {
    const fp = await fresh('a.op');
    await handleAddImagePlaceholderV0({ filePath: fp, label: 'Upload cover' });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[1].type).toBe('text');
    expect(kids[1].content).toBe('Upload cover');
    expect(kids[1].role).toBe('image-placeholder-label');
  });

  it('custom icon + size + corner_radius honored (clamp 40 on size)', async () => {
    const fp = await fresh('a.op');
    await handleAddImagePlaceholderV0({
      filePath: fp,
      width: 20,
      height: 25,
      icon: 'video',
      corner_radius: 24,
    });
    const ph = getRoot(await readDoc(fp));
    expect(ph.width).toBe(40); // clamped
    expect(ph.height).toBe(40); // clamped
    expect(ph.cornerRadius).toBe(24);
    const kids = ph.children as Record<string, unknown>[];
    expect(kids[0].iconFontName).toBe('video');
  });

  it('uses frame+fill, NEVER image node (empty-image render avoidance)', async () => {
    const fp = await fresh('a.op');
    await handleAddImagePlaceholderV0({ filePath: fp });
    const ph = getRoot(await readDoc(fp));
    expect(ph.type).toBe('frame');
    expect(ph.type).not.toBe('image');
    const fill = ph.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#F1F5F9');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddImagePlaceholderV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });

  it('image_search_query is stamped onto the frame as imageSearchQuery', async () => {
    const fp = await fresh('a.op');
    await handleAddImagePlaceholderV0({
      filePath: fp,
      image_search_query: 'burger fries',
    });
    const ph = getRoot(await readDoc(fp));
    expect(ph.imageSearchQuery).toBe('burger fries');
  });

  it('omitting image_search_query leaves the field undefined', async () => {
    const fp = await fresh('a.op');
    await handleAddImagePlaceholderV0({ filePath: fp });
    const ph = getRoot(await readDoc(fp));
    expect(ph.imageSearchQuery).toBeUndefined();
  });

  it('empty-string image_search_query is treated as missing', async () => {
    const fp = await fresh('a.op');
    await handleAddImagePlaceholderV0({ filePath: fp, image_search_query: '' });
    const ph = getRoot(await readDoc(fp));
    expect(ph.imageSearchQuery).toBeUndefined();
  });
});
