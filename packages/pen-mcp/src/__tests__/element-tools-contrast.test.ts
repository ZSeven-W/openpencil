/**
 * Foreground-contrast regression guard. pen-core's DEFAULT_FILL is
 * `#d1d5db` (gray-300) — so any text/icon without an explicit `fill`
 * renders as light gray. On a colored background (primary blue, iOS green,
 * dark #111827, bright white), gray-300 produces unreadable content. The
 * tools below hardcode colored fills on the container, so they MUST also
 * set the foreground color — otherwise the weak-model design output ships
 * broken by default.
 *
 * This test locks the foreground colors so the regression can't sneak
 * back in via an unrelated edit.
 */
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { handleAddToastV0 } from '../tools/add-toast-v0';
import { handleAddFabV0 } from '../tools/add-fab-v0';
import { handleAddStepperV0 } from '../tools/add-stepper-v0';
import { handleAddCheckboxV0 } from '../tools/add-checkbox-v0';
import { handleAddSegmentedControlV0 } from '../tools/add-segmented-control-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-element-contrast');
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
function firstFillColor(node: Record<string, unknown>): string | undefined {
  const fills = node.fill as Array<{ color?: string }> | undefined;
  return fills?.[0]?.color;
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

describe('element tools — colored backgrounds ship with readable foreground', () => {
  it('toast: white text + icon on dark #111827 bg', async () => {
    const fp = await fresh('a.op');
    await handleAddToastV0({ filePath: fp, message: 'Copied', icon: 'check' });
    const t = getRoot(await readDoc(fp));
    expect(firstFillColor(t)).toBe('#111827');
    const kids = t.children as Record<string, unknown>[];
    expect(firstFillColor(kids[0])).toBe('#FFFFFF'); // icon
    expect(firstFillColor(kids[1])).toBe('#FFFFFF'); // message
  });

  it('fab: white icon on blue #2563EB bg', async () => {
    const fp = await fresh('a.op');
    await handleAddFabV0({ filePath: fp, icon: 'plus' });
    const fab = getRoot(await readDoc(fp));
    expect(firstFillColor(fab)).toBe('#2563EB');
    const icon = (fab.children as Record<string, unknown>[])[0];
    expect(firstFillColor(icon)).toBe('#FFFFFF');
  });

  it('stepper: done numbers white on blue, pending numbers gray-500 on gray-200', async () => {
    const fp = await fresh('a.op');
    await handleAddStepperV0({ filePath: fp, total: 3, current: 1 });
    const steps = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    // steps 0 + 2 are done/active (blue), step 4 (index=4 in children array with
    // connectors interleaved) is pending
    const done0 = steps[0];
    const pending = steps[4];
    expect(firstFillColor(done0)).toBe('#2563EB');
    expect(firstFillColor(pending)).toBe('#E5E7EB');
    const doneNum = (done0.children as Record<string, unknown>[])[0];
    const pendingNum = (pending.children as Record<string, unknown>[])[0];
    expect(firstFillColor(doneNum)).toBe('#FFFFFF');
    expect(firstFillColor(pendingNum)).toBe('#6B7280');
  });

  it('checkbox: white check icon on blue-filled box when checked', async () => {
    const fp = await fresh('a.op');
    await handleAddCheckboxV0({ filePath: fp, label: 'Done', checked: true });
    const box = (getRoot(await readDoc(fp)).children as Record<string, unknown>[])[0];
    expect(firstFillColor(box)).toBe('#2563EB');
    const check = (box.children as Record<string, unknown>[])[0];
    expect(firstFillColor(check)).toBe('#FFFFFF');
  });

  it('segmented control: active label dark-900 (on white), inactive label gray-600 (on gray-100)', async () => {
    const fp = await fresh('a.op');
    await handleAddSegmentedControlV0({
      filePath: fp,
      items: [{ label: 'Day' }, { label: 'Week', active: true }],
    });
    const segs = getRoot(await readDoc(fp)).children as Record<string, unknown>[];
    const inactiveLabel = (segs[0].children as Record<string, unknown>[])[0];
    const activeLabel = (segs[1].children as Record<string, unknown>[])[0];
    expect(firstFillColor(inactiveLabel)).toBe('#4B5563');
    expect(firstFillColor(activeLabel)).toBe('#111827');
  });
});
