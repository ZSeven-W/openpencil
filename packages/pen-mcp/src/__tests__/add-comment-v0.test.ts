import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddCommentV0 } from '../tools/add-comment-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-comment-v0');
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

describe('add_comment_v0', () => {
  it('registered; required author + body', () => {
    expect(DESIGN_TOOL_NAMES.has('add_comment_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_comment_v0');
    expect(def?.inputSchema.required).toEqual(['author', 'body']);
  });

  it('minimal: avatar (blank) + author + body (no timestamp)', async () => {
    const fp = await fresh('a.op');
    await handleAddCommentV0({ filePath: fp, author: 'Alice', body: 'Looks great!' });
    const c = getRoot(await readDoc(fp));
    expect(c.role).toBe('comment');
    expect(c.layout).toBe('horizontal');
    expect(c.alignItems).toBe('start');
    const kids = c.children as Record<string, unknown>[];
    expect(kids.length).toBe(2); // avatar + body column
    // Avatar: empty (no initial)
    const avatar = kids[0] as Record<string, unknown>;
    expect(avatar.role).toBe('comment-avatar');
    expect(avatar.cornerRadius).toBe(20); // default 40/2
    const avatarKids = avatar.children as Record<string, unknown>[];
    expect(avatarKids.length).toBe(0);
    // Body column: header + body text
    const bodyCol = kids[1] as Record<string, unknown>;
    expect(bodyCol.role).toBe('comment-body-column');
    const bodyKids = bodyCol.children as Record<string, unknown>[];
    expect(bodyKids.length).toBe(2); // header + body
    const header = bodyKids[0] as Record<string, unknown>;
    const headerKids = header.children as Record<string, unknown>[];
    expect(headerKids.length).toBe(1); // just author, no timestamp
    expect(headerKids[0].content).toBe('Alice');
  });

  it('avatar_initial renders centered text in the avatar circle', async () => {
    const fp = await fresh('a.op');
    await handleAddCommentV0({
      filePath: fp,
      author: 'Bob',
      body: 'Nice',
      avatar_initial: 'bd',
    });
    const avatar = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    const avatarKids = avatar.children as Record<string, unknown>[];
    expect(avatarKids.length).toBe(1);
    // Upper-cased + truncated to 2
    expect(avatarKids[0].content).toBe('BD');
  });

  it('timestamp adds a second text to the header row', async () => {
    const fp = await fresh('a.op');
    await handleAddCommentV0({
      filePath: fp,
      author: 'Alice',
      body: 'Hi',
      timestamp: '2h ago',
    });
    const bodyCol = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const header = (bodyCol.children as Record<string, unknown>[])[0];
    const headerKids = header.children as Record<string, unknown>[];
    expect(headerKids.length).toBe(2);
    expect(headerKids[1].content).toBe('2h ago');
    expect(headerKids[1].role).toBe('comment-timestamp');
  });

  it('avatar_size honored (clamped to >=24)', async () => {
    const fp = await fresh('a.op');
    await handleAddCommentV0({
      filePath: fp,
      author: 'X',
      body: 'Y',
      avatar_size: 10,
    });
    const avatar = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(avatar.width).toBe(24);
    expect(avatar.cornerRadius).toBe(12);
  });

  it('every node has a unique id', async () => {
    const fp = await fresh('a.op');
    await handleAddCommentV0({
      filePath: fp,
      author: 'Alice',
      body: 'Nice',
      avatar_initial: 'A',
      timestamp: '2h',
    });
    const ids: string[] = [];
    function walk(n: Record<string, unknown>): void {
      if (typeof n.id === 'string') ids.push(n.id);
      if (Array.isArray(n.children))
        (n.children as Record<string, unknown>[]).forEach(
          (c) => c && typeof c === 'object' && walk(c),
        );
    }
    walk(getRoot(await readDoc(fp)));
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids.length).toBeGreaterThanOrEqual(6);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddCommentV0({ filePath: fp, author: 'X', body: 'Y', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
