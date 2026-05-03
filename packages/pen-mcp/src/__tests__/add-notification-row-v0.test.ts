import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddNotificationRowV0 } from '../tools/add-notification-row-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-notification-row-v0');
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

describe('add_notification_row_v0', () => {
  it('registered; required title', () => {
    expect(DESIGN_TOOL_NAMES.has('add_notification_row_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_notification_row_v0');
    expect(def?.inputSchema.required).toEqual(['title']);
  });

  it('minimal: icon + title only (no timestamp, no body, no unread)', async () => {
    const fp = await fresh('a.op');
    await handleAddNotificationRowV0({ filePath: fp, title: 'Welcome' });
    const n = getRoot(await readDoc(fp));
    expect(n.role).toBe('notification-row');
    const kids = n.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    expect(kids[0].iconFontName).toBe('bell');
    const bodyCol = kids[1];
    const bcKids = bodyCol.children as Record<string, unknown>[];
    expect(bcKids.length).toBe(1); // only header, no body
    const header = bcKids[0];
    const headerKids = header.children as Record<string, unknown>[];
    expect(headerKids.length).toBe(1); // only title row, no timestamp
    const titleRow = headerKids[0];
    const trKids = titleRow.children as Record<string, unknown>[];
    expect(trKids.length).toBe(1); // only title, no unread dot
    expect(trKids[0].content).toBe('Welcome');
  });

  it('unread=true adds red dot next to title', async () => {
    const fp = await fresh('a.op');
    await handleAddNotificationRowV0({ filePath: fp, title: 'X', unread: true });
    const bodyCol = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const header = (bodyCol.children as Record<string, unknown>[])[0];
    const titleRow = (header.children as Record<string, unknown>[])[0];
    const trKids = titleRow.children as Record<string, unknown>[];
    expect(trKids.length).toBe(2);
    const dot = trKids[1];
    expect(dot.role).toBe('notification-unread-dot');
    const fill = dot.fill as Array<{ color: string }>;
    expect(fill[0].color).toBe('#EF4444');
  });

  it('timestamp adds text to header row end', async () => {
    const fp = await fresh('a.op');
    await handleAddNotificationRowV0({ filePath: fp, title: 'X', timestamp: '2h' });
    const bodyCol = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const header = (bodyCol.children as Record<string, unknown>[])[0];
    const headerKids = header.children as Record<string, unknown>[];
    expect(headerKids.length).toBe(2);
    expect(headerKids[1].role).toBe('notification-timestamp');
  });

  it('body adds preview line below header', async () => {
    const fp = await fresh('a.op');
    await handleAddNotificationRowV0({
      filePath: fp,
      title: 'Comment',
      body: 'Alice commented on your post.',
    });
    const bodyCol = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[1];
    const bcKids = bodyCol.children as Record<string, unknown>[];
    expect(bcKids.length).toBe(2);
    expect(bcKids[1].role).toBe('notification-body');
  });

  it('custom icon honored', async () => {
    const fp = await fresh('a.op');
    await handleAddNotificationRowV0({ filePath: fp, title: 'New msg', icon: 'message-circle' });
    const icon = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(icon.iconFontName).toBe('message-circle');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddNotificationRowV0({ filePath: fp, title: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
