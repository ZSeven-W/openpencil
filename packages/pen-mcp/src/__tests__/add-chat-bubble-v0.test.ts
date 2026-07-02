import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddChatBubbleV0 } from '../tools/add-chat-bubble-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-chat-bubble-v0');
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
function surfaceColor(root: Record<string, unknown>): string | undefined {
  const surface = findByRole(root, 'chat-bubble-surface')!;
  const fills = surface.fill as Array<{ color?: string }> | undefined;
  return fills?.[0]?.color;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['c.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_chat_bubble_v0', () => {
  it('registered; required=[message]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_chat_bubble_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_chat_bubble_v0');
    expect(def?.inputSchema.required).toEqual(['message']);
  });

  it('default side=left: slate-100 surface, no author, flex-start align', async () => {
    const fp = await fresh('c.op');
    await handleAddChatBubbleV0({ filePath: fp, message: 'Hello!' });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('chat-bubble-left');
    expect(root.alignItems).toBe('flex-start');
    expect(surfaceColor(root)).toBe('#F1F5F9');
    expect(findByRole(root, 'chat-bubble-author')).toBeUndefined();
    expect(findByRole(root, 'chat-bubble-message')!.content).toBe('Hello!');
  });

  it('side=left with author shows author text above bubble', async () => {
    const fp = await fresh('c.op');
    await handleAddChatBubbleV0({
      filePath: fp,
      message: 'Hi there',
      author: 'Sarah',
    });
    const root = getRoot(await readDoc(fp));
    const author = findByRole(root, 'chat-bubble-author')!;
    expect(author.content).toBe('Sarah');
    // Author must come BEFORE surface in children
    const kids = root.children as Record<string, unknown>[];
    expect(kids[0].role).toBe('chat-bubble-author');
    expect(kids[1].role).toBe('chat-bubble-surface');
  });

  it('side=right: accent surface, no author even when given, flex-end align', async () => {
    const fp = await fresh('c.op');
    await handleAddChatBubbleV0({
      filePath: fp,
      message: 'Sounds good!',
      side: 'right',
      author: 'IgnoredForSelf', // intentionally provided; should be dropped
    });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('chat-bubble-right');
    expect(root.alignItems).toBe('flex-end');
    expect(surfaceColor(root)).toBe('#2563EB'); // default accent
    expect(findByRole(root, 'chat-bubble-author')).toBeUndefined();
  });

  it('side=right with custom accent_color', async () => {
    const fp = await fresh('c.op');
    await handleAddChatBubbleV0({
      filePath: fp,
      message: 'Done.',
      side: 'right',
      accent_color: '#10B981',
    });
    const root = getRoot(await readDoc(fp));
    expect(surfaceColor(root)).toBe('#10B981');
  });

  it('timestamp shown below bubble on both sides', async () => {
    const fp = await fresh('c.op');
    await handleAddChatBubbleV0({
      filePath: fp,
      message: 'x',
      timestamp: '2m',
    });
    const root = getRoot(await readDoc(fp));
    const ts = findByRole(root, 'chat-bubble-timestamp')!;
    expect(ts.content).toBe('2m');
  });

  it('max_width clamps below to 160', async () => {
    const fp = await fresh('c.op');
    await handleAddChatBubbleV0({ filePath: fp, message: 'x', max_width: 50 });
    const root = getRoot(await readDoc(fp));
    const surface = findByRole(root, 'chat-bubble-surface')!;
    expect(surface.width).toBe(160);
  });

  it('max_width clamps above to 480', async () => {
    const fp = await fresh('c.op');
    await handleAddChatBubbleV0({ filePath: fp, message: 'x', max_width: 2000 });
    const root = getRoot(await readDoc(fp));
    const surface = findByRole(root, 'chat-bubble-surface')!;
    expect(surface.width).toBe(480);
  });

  it('message text uses textGrowth=fixed-width for wrapping', async () => {
    const fp = await fresh('c.op');
    await handleAddChatBubbleV0({
      filePath: fp,
      message: 'A very long message that should wrap within the bubble surface.',
    });
    const root = getRoot(await readDoc(fp));
    const msg = findByRole(root, 'chat-bubble-message')!;
    expect(msg.textGrowth).toBe('fixed-width');
    expect(msg.width).toBe('fill_container');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('c.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddChatBubbleV0({ filePath: fp, message: 'x', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
