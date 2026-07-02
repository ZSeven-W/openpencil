import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddDatePickerV0 } from '../tools/add-date-picker-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-date-picker-v0');
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

describe('add_date_picker_v0', () => {
  it('registered; required=[label]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_date_picker_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_date_picker_v0');
    expect(def?.inputSchema.required).toEqual(['label']);
  });

  it('placeholder state: muted placeholder + calendar icon only', async () => {
    const fp = await fresh('a.op');
    await handleAddDatePickerV0({ filePath: fp, label: 'Due date' });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('date-picker');
    const input = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'date-picker-input',
    )!;
    const left = (input.children as Record<string, unknown>[])[0];
    expect(left.role).toBe('date-picker-placeholder');
    expect(left.content).toBe('Select date');
    // slate-400 for placeholder
    const placeholderFill = left.fill as Array<{ color: string }>;
    expect(placeholderFill[0].color).toBe('#94A3B8');
  });

  it('populated state: value text in slate-900, role=date-picker-value', async () => {
    const fp = await fresh('a.op');
    await handleAddDatePickerV0({
      filePath: fp,
      label: 'Start date',
      value: 'Jan 15, 2026',
    });
    const root = getRoot(await readDoc(fp));
    const input = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'date-picker-input',
    )!;
    const left = (input.children as Record<string, unknown>[])[0];
    expect(left.role).toBe('date-picker-value');
    expect(left.content).toBe('Jan 15, 2026');
    const valueFill = left.fill as Array<{ color: string }>;
    expect(valueFill[0].color).toBe('#0F172A');
  });

  it('clearable=true AND value present → X icon inserted before calendar', async () => {
    const fp = await fresh('a.op');
    await handleAddDatePickerV0({
      filePath: fp,
      label: 'Date',
      value: 'Feb 1, 2026',
      clearable: true,
    });
    const root = getRoot(await readDoc(fp));
    const input = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'date-picker-input',
    )!;
    const right = (input.children as Record<string, unknown>[])[1];
    const rightKids = right.children as Record<string, unknown>[];
    expect(rightKids.length).toBe(2);
    expect(rightKids[0].role).toBe('date-picker-clear');
    expect(rightKids[0].iconFontName).toBe('x');
    expect(rightKids[1].iconFontName).toBe('calendar');
  });

  it('clearable=true BUT no value → no X icon (can only clear what exists)', async () => {
    const fp = await fresh('a.op');
    await handleAddDatePickerV0({
      filePath: fp,
      label: 'Date',
      clearable: true,
    });
    const root = getRoot(await readDoc(fp));
    const input = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'date-picker-input',
    )!;
    const right = (input.children as Record<string, unknown>[])[1];
    const rightKids = right.children as Record<string, unknown>[];
    expect(rightKids.length).toBe(1);
    expect(rightKids[0].iconFontName).toBe('calendar');
  });

  it('custom placeholder wins over default', async () => {
    const fp = await fresh('a.op');
    await handleAddDatePickerV0({
      filePath: fp,
      label: 'Birthday',
      placeholder: 'Pick your birthday',
    });
    const root = getRoot(await readDoc(fp));
    const input = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'date-picker-input',
    )!;
    const left = (input.children as Record<string, unknown>[])[0];
    expect(left.content).toBe('Pick your birthday');
  });

  it('required=true → label gets " *"', async () => {
    const fp = await fresh('a.op');
    await handleAddDatePickerV0({ filePath: fp, label: 'Date', required: true });
    const root = getRoot(await readDoc(fp));
    const label = (root.children as Record<string, unknown>[]).find(
      (k) => k.role === 'date-picker-label',
    )!;
    expect(label.content).toBe('Date *');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddDatePickerV0({ filePath: fp, label: 'Date', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
