import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddStatusBadgeV0 } from '../tools/add-status-badge-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-status-badge-v0');
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

describe('add_status_badge_v0', () => {
  it('registered; required label, tone enum', () => {
    expect(DESIGN_TOOL_NAMES.has('add_status_badge_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_status_badge_v0');
    expect(def?.inputSchema.required).toEqual(['label']);
    const toneProp = (def?.inputSchema.properties as unknown as Record<string, { enum?: string[] }>)
      ?.tone;
    expect(toneProp?.enum).toEqual(['success', 'warning', 'error', 'info', 'neutral']);
  });

  it('default tone (neutral): slate-400 dot + label', async () => {
    const fp = await fresh('a.op');
    await handleAddStatusBadgeV0({ filePath: fp, label: 'Idle' });
    const badge = getRoot(await readDoc(fp));
    expect(badge.role).toBe('status-badge');
    expect(badge.layout).toBe('horizontal');
    const kids = badge.children as Record<string, unknown>[];
    expect(kids.length).toBe(2);
    const dot = kids[0];
    expect(dot.role).toBe('status-dot');
    expect(dot.cornerRadius).toBe(4);
    expect(dot.width).toBe(8);
    const dotFill = dot.fill as Array<{ color: string }>;
    expect(dotFill[0].color).toBe('#94A3B8'); // slate-400
    expect(kids[1].content).toBe('Idle');
  });

  it('tone=success → emerald dot', async () => {
    const fp = await fresh('a.op');
    await handleAddStatusBadgeV0({ filePath: fp, label: 'Online', tone: 'success' });
    const dot = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    const dotFill = dot.fill as Array<{ color: string }>;
    expect(dotFill[0].color).toBe('#10B981');
  });

  it('tone=warning → amber', async () => {
    const fp = await fresh('a.op');
    await handleAddStatusBadgeV0({ filePath: fp, label: 'Degraded', tone: 'warning' });
    const dot = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    const dotFill = dot.fill as Array<{ color: string }>;
    expect(dotFill[0].color).toBe('#F59E0B');
  });

  it('tone=error → red', async () => {
    const fp = await fresh('a.op');
    await handleAddStatusBadgeV0({ filePath: fp, label: 'Down', tone: 'error' });
    const dot = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    const dotFill = dot.fill as Array<{ color: string }>;
    expect(dotFill[0].color).toBe('#EF4444');
  });

  it('tone=info → blue', async () => {
    const fp = await fresh('a.op');
    await handleAddStatusBadgeV0({ filePath: fp, label: 'New', tone: 'info' });
    const dot = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    const dotFill = dot.fill as Array<{ color: string }>;
    expect(dotFill[0].color).toBe('#3B82F6');
  });

  it('dot is a FRAME with cornerRadius=4 (not an ellipse — anti-pattern avoidance)', async () => {
    // 8×8 pill via frame+cornerRadius, NOT a tiny ellipse. Ellipse
    // at that size is the classic "status dot = stacked ellipses"
    // bait for rewriteLlmAntiPatterns. This test locks the frame
    // approach in place.
    const fp = await fresh('a.op');
    await handleAddStatusBadgeV0({ filePath: fp, label: 'X' });
    const dot = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(dot.type).toBe('frame');
    expect(dot.type).not.toBe('ellipse');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddStatusBadgeV0({ filePath: fp, label: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
