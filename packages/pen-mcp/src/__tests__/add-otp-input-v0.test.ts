import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddOtpInputV0 } from '../tools/add-otp-input-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-otp-input-v0');
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
  for (const f of ['o.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_otp_input_v0', () => {
  it('registered; required=[] (all optional)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_otp_input_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_otp_input_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('defaults: 6 slots, first focused (accent outline), rest empty', async () => {
    const fp = await fresh('o.op');
    await handleAddOtpInputV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('otp-input');
    const slots = root.children as Record<string, unknown>[];
    expect(slots.length).toBe(6);
    expect(slots[0].role).toBe('otp-slot-focused');
    expect(slots[1].role).toBe('otp-slot');
    // Focused slot has 2px stroke, accent color
    const focusedStroke = slots[0].stroke as { thickness: number; fill: Array<{ color: string }> };
    expect(focusedStroke.thickness).toBe(2);
    expect(focusedStroke.fill[0].color).toBe('#2563EB');
  });

  it('partial state: filled slots have digit text + role=otp-slot-filled', async () => {
    const fp = await fresh('o.op');
    await handleAddOtpInputV0({
      filePath: fp,
      length: 6,
      digits: ['1', '2', '3'],
      focused_index: 3,
    });
    const root = getRoot(await readDoc(fp));
    const slots = root.children as Record<string, unknown>[];
    // Slots 0..2 filled, 3 focused, 4..5 empty
    expect(slots[0].role).toBe('otp-slot-filled');
    expect(slots[1].role).toBe('otp-slot-filled');
    expect(slots[2].role).toBe('otp-slot-filled');
    expect(slots[3].role).toBe('otp-slot-focused');
    expect(slots[4].role).toBe('otp-slot');
    // Filled slots have a text child with the digit
    const filledKids = slots[0].children as Record<string, unknown>[];
    expect(filledKids.length).toBe(1);
    expect(filledKids[0].content).toBe('1');
    expect(filledKids[0].role).toBe('otp-digit');
  });

  it('full state: all slots filled, none focused', async () => {
    const fp = await fresh('o.op');
    await handleAddOtpInputV0({
      filePath: fp,
      length: 4,
      digits: ['1', '2', '3', '4'],
      focused_index: 0, // even though index=0, slot 0 is filled so takes filled role
    });
    const root = getRoot(await readDoc(fp));
    const slots = root.children as Record<string, unknown>[];
    expect(slots.length).toBe(4);
    for (const s of slots) {
      expect(s.role).toBe('otp-slot-filled');
    }
  });

  it('length clamps below to 4', async () => {
    const fp = await fresh('o.op');
    await handleAddOtpInputV0({ filePath: fp, length: 2 });
    const root = getRoot(await readDoc(fp));
    expect((root.children as unknown[]).length).toBe(4);
  });

  it('length clamps above to 8', async () => {
    const fp = await fresh('o.op');
    await handleAddOtpInputV0({ filePath: fp, length: 20 });
    const root = getRoot(await readDoc(fp));
    expect((root.children as unknown[]).length).toBe(8);
  });

  it('accent_color override changes the focused-slot border', async () => {
    const fp = await fresh('o.op');
    await handleAddOtpInputV0({ filePath: fp, accent_color: '#EF4444' });
    const root = getRoot(await readDoc(fp));
    const focused = (root.children as Record<string, unknown>[])[0];
    const stroke = focused.stroke as { fill: Array<{ color: string }> };
    expect(stroke.fill[0].color).toBe('#EF4444');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('o.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddOtpInputV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
