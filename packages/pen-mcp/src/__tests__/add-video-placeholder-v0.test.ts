import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddVideoPlaceholderV0 } from '../tools/add-video-placeholder-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-video-placeholder-v0');
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

describe('add_video_placeholder_v0', () => {
  it('registered; no required params', () => {
    expect(DESIGN_TOOL_NAMES.has('add_video_placeholder_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_video_placeholder_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('default: 320×180 (16:9), dark slate fill, white play icon', async () => {
    const fp = await fresh('a.op');
    await handleAddVideoPlaceholderV0({ filePath: fp });
    const v = getRoot(await readDoc(fp));
    expect(v.role).toBe('video-placeholder');
    expect(v.width).toBe(320);
    expect(v.height).toBe(180);
    expect(v.cornerRadius).toBe(12);
    const fill = v.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#334155');
    const kids = v.children as Record<string, unknown>[];
    expect(kids.length).toBe(1);
    expect(kids[0].iconFontName).toBe('play');
    const iconFill = kids[0].fill as Array<{ color: string }>;
    expect(iconFill[0].color).toBe('#FFFFFF');
  });

  it('label adds a second text child with white @ 70%', async () => {
    const fp = await fresh('a.op');
    await handleAddVideoPlaceholderV0({ filePath: fp, label: 'Coming soon' });
    const kids = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[1].content).toBe('Coming soon');
    expect(kids[1].role).toBe('video-placeholder-label');
    const labelFill = kids[1].fill as Array<{ color: string }>;
    expect(labelFill[0].color).toBe('#FFFFFFB3');
  });

  it('size clamped to mins', async () => {
    const fp = await fresh('a.op');
    await handleAddVideoPlaceholderV0({ filePath: fp, width: 10, height: 10 });
    const v = getRoot(await readDoc(fp));
    expect(v.width).toBe(80);
    expect(v.height).toBe(60);
  });

  it('uses frame+fill + icon_font, NEVER a video-shaped rectangle with play path', async () => {
    // Regression guard: the "play triangle via path" approach is a
    // classic LLM anti-pattern for video placeholders. We use a
    // dedicated lucide `play` icon_font instead.
    const fp = await fresh('a.op');
    await handleAddVideoPlaceholderV0({ filePath: fp });
    const v = getRoot(await readDoc(fp));
    expect(v.type).toBe('frame');
    const kids = v.children as Record<string, unknown>[];
    expect(kids[0].type).toBe('icon_font');
    expect(kids[0].type).not.toBe('path');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddVideoPlaceholderV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
