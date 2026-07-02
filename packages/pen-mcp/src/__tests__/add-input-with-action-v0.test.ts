import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddInputWithActionV0 } from '../tools/add-input-with-action-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-input-with-action-v0');
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
function fillColor(n: Record<string, unknown> | undefined): string | undefined {
  const fills = n?.fill as Array<{ color?: string }> | undefined;
  return fills?.[0]?.color;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['i.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_input_with_action_v0', () => {
  it('registered; required=[placeholder]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_input_with_action_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_input_with_action_v0');
    expect(def?.inputSchema.required).toEqual(['placeholder']);
  });

  it('default text variant: pill button with "Submit" label and accent fill', async () => {
    const fp = await fresh('i.op');
    await handleAddInputWithActionV0({ filePath: fp, placeholder: 'Enter email' });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('input-with-action');
    const button = findByRole(root, 'input-with-action-button')!;
    expect(button.width).toBe('fit_content');
    expect(fillColor(button)).toBe('#2563EB');
    expect(findByRole(root, 'input-with-action-label')!.content).toBe('Submit');
  });

  it('custom action_label renders as button text', async () => {
    const fp = await fresh('i.op');
    await handleAddInputWithActionV0({
      filePath: fp,
      placeholder: 'Enter email',
      action_label: 'Subscribe',
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'input-with-action-label')!.content).toBe('Subscribe');
  });

  it('icon variant: 44x44 square button with action icon', async () => {
    const fp = await fresh('i.op');
    await handleAddInputWithActionV0({
      filePath: fp,
      placeholder: 'Type a message',
      action_kind: 'icon',
      action_icon: 'send',
    });
    const root = getRoot(await readDoc(fp));
    const button = findByRole(root, 'input-with-action-button')!;
    expect(button.width).toBe(44);
    expect(button.height).toBe(44);
    expect(findByRole(root, 'input-with-action-icon')!.iconFontName).toBe('send');
    expect(findByRole(root, 'input-with-action-label')).toBeUndefined();
  });

  it('icon variant defaults to arrow-right when action_icon omitted', async () => {
    const fp = await fresh('i.op');
    await handleAddInputWithActionV0({
      filePath: fp,
      placeholder: 'Search',
      action_kind: 'icon',
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'input-with-action-icon')!.iconFontName).toBe('arrow-right');
  });

  it('value renders populated state in slate-900', async () => {
    const fp = await fresh('i.op');
    await handleAddInputWithActionV0({
      filePath: fp,
      placeholder: 'Enter email',
      value: 'user@example.com',
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'input-with-action-text')!.content).toBe('user@example.com');
    expect(fillColor(findByRole(root, 'input-with-action-text'))).toBe('#0F172A');
  });

  it('placeholder state has slate-400 muted color', async () => {
    const fp = await fresh('i.op');
    await handleAddInputWithActionV0({ filePath: fp, placeholder: 'Enter email' });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(findByRole(root, 'input-with-action-text'))).toBe('#94A3B8');
  });

  it('leading icon renders inside input frame on the left', async () => {
    const fp = await fresh('i.op');
    await handleAddInputWithActionV0({
      filePath: fp,
      placeholder: 'Enter email',
      leading_icon: 'mail',
    });
    const root = getRoot(await readDoc(fp));
    const leadingIcon = findByRole(root, 'input-with-action-leading-icon')!;
    expect(leadingIcon.iconFontName).toBe('mail');
    // Leading icon is BEFORE the text in the input frame
    const inputFrame = findByRole(root, 'input-with-action-input')!;
    const inputKids = inputFrame.children as Array<Record<string, unknown>>;
    expect(inputKids[0].role).toBe('input-with-action-leading-icon');
    expect(inputKids[1].role).toBe('input-with-action-text');
  });

  it('row height is 44px (matches form-field standard)', async () => {
    const fp = await fresh('i.op');
    await handleAddInputWithActionV0({ filePath: fp, placeholder: 'X' });
    const root = getRoot(await readDoc(fp));
    expect(root.height).toBe(44);
    expect(findByRole(root, 'input-with-action-input')!.height).toBe(44);
  });

  it('width clamps (< 280 → 280)', async () => {
    const fp = await fresh('i.op');
    await handleAddInputWithActionV0({ filePath: fp, placeholder: 'X', width: 100 });
    const root = getRoot(await readDoc(fp));
    expect(root.width).toBe(280);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('i.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddInputWithActionV0({ filePath: fp, placeholder: 'X', parent_id: 'nope' }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
