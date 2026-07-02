import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddPhoneInputV0 } from '../tools/add-phone-input-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-phone-input-v0');
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
  for (const f of ['p.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_phone_input_v0', () => {
  it('registered; no required fields (all defaults)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_phone_input_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_phone_input_v0');
    const req = def?.inputSchema.required as string[] | undefined;
    expect(req === undefined || req.length === 0).toBe(true);
  });

  it('defaults: country=+1, placeholder text, no label, slate-400 (placeholder) digits color', async () => {
    const fp = await fresh('p.op');
    await handleAddPhoneInputV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('phone-input-field');
    expect(findByRole(root, 'phone-input-code')!.content).toBe('+1');
    expect(findByRole(root, 'phone-input-digits-text')!.content).toBe('(555) 555-5555');
    expect(fillColor(findByRole(root, 'phone-input-digits-text'))).toBe('#94A3B8');
    expect(findByRole(root, 'form-label')).toBeUndefined();
    expect(findByRole(root, 'phone-input-flag')).toBeUndefined();
  });

  it('with value renders populated state in slate-900', async () => {
    const fp = await fresh('p.op');
    await handleAddPhoneInputV0({ filePath: fp, value: '555 123 4567' });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'phone-input-digits-text')!.content).toBe('555 123 4567');
    expect(fillColor(findByRole(root, 'phone-input-digits-text'))).toBe('#0F172A');
  });

  it('label appears with required asterisk', async () => {
    const fp = await fresh('p.op');
    await handleAddPhoneInputV0({ filePath: fp, label: 'Phone number', required: true });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'form-label')!.content).toBe('Phone number *');
  });

  it('country flag renders before code when provided', async () => {
    const fp = await fresh('p.op');
    await handleAddPhoneInputV0({
      filePath: fp,
      country_code: '+86',
      country_flag: '🇨🇳',
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'phone-input-flag')!.content).toBe('🇨🇳');
    expect(findByRole(root, 'phone-input-code')!.content).toBe('+86');
    // Country block has flag before code (children order)
    const country = findByRole(root, 'phone-input-country')!;
    const countryKids = country.children as Array<Record<string, unknown>>;
    expect(countryKids[0].role).toBe('phone-input-flag');
    expect(countryKids[1].role).toBe('phone-input-code');
  });

  it('always emits a chevron-down icon for the country selector', async () => {
    const fp = await fresh('p.op');
    await handleAddPhoneInputV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    const chevron = findByRole(root, 'phone-input-chevron')!;
    expect(chevron.iconFontName).toBe('chevron-down');
    expect(chevron.iconFontFamily).toBe('lucide');
  });

  it('always emits the divider rectangle between country and digits', async () => {
    const fp = await fresh('p.op');
    await handleAddPhoneInputV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    const divider = findByRole(root, 'phone-input-divider')!;
    expect(divider.type).toBe('rectangle');
    expect(divider.width).toBe(1);
  });

  it('row height is 44px (matches form-field standard)', async () => {
    const fp = await fresh('p.op');
    await handleAddPhoneInputV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'phone-input-row')!.height).toBe(44);
  });

  it('width clamps (< 240 → 240)', async () => {
    const fp = await fresh('p.op');
    await handleAddPhoneInputV0({ filePath: fp, width: 100 });
    const root = getRoot(await readDoc(fp));
    expect(root.width).toBe(240);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('p.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddPhoneInputV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
