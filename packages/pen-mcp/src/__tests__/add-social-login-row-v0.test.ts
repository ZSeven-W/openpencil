import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddSocialLoginRowV0 } from '../tools/add-social-login-row-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-social-login-row-v0');
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
function findAllByRole(n: Record<string, unknown>, role: string): Record<string, unknown>[] {
  const out: Record<string, unknown>[] = [];
  if (n.role === role) out.push(n);
  const kids = (n.children ?? []) as Record<string, unknown>[];
  for (const c of kids) out.push(...findAllByRole(c, role));
  return out;
}
function findByRole(n: Record<string, unknown>, role: string): Record<string, unknown> | undefined {
  return findAllByRole(n, role)[0];
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['s.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_social_login_row_v0', () => {
  it('registered; required=[providers]', () => {
    expect(DESIGN_TOOL_NAMES.has('add_social_login_row_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_social_login_row_v0');
    expect(def?.inputSchema.required).toEqual(['providers']);
  });

  it('default orientation=vertical → full-width 48px buttons with icon + "Continue with X" label', async () => {
    const fp = await fresh('s.op');
    await handleAddSocialLoginRowV0({
      filePath: fp,
      providers: [{ name: 'google' }, { name: 'apple' }, { name: 'microsoft' }],
    });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('social-login-row');
    expect(root.layout).toBe('vertical');

    const buttons = findAllByRole(root, 'social-login-button');
    expect(buttons).toHaveLength(3);
    expect(buttons[0].width).toBe('fill_container');
    expect(buttons[0].height).toBe(48);

    const labels = findAllByRole(root, 'social-login-button-label');
    expect(labels.map((l) => l.content)).toEqual([
      'Continue with Google',
      'Continue with Apple',
      'Continue with Microsoft',
    ]);
  });

  it('horizontal orientation → 48×48 icon-only compact pills, no labels', async () => {
    const fp = await fresh('s.op');
    await handleAddSocialLoginRowV0({
      filePath: fp,
      providers: [{ name: 'google' }, { name: 'github' }],
      orientation: 'horizontal',
    });
    const root = getRoot(await readDoc(fp));
    expect(root.layout).toBe('horizontal');

    const compactButtons = findAllByRole(root, 'social-login-button-compact');
    expect(compactButtons).toHaveLength(2);
    expect(compactButtons[0].width).toBe(48);
    expect(compactButtons[0].height).toBe(48);

    // No labels in horizontal variant
    expect(findAllByRole(root, 'social-login-button-label')).toHaveLength(0);
    // Full-size "button" role not used for compact
    expect(findAllByRole(root, 'social-login-button')).toHaveLength(0);
  });

  it('known provider names map to lucide icons (google → chrome, github → github)', async () => {
    const fp = await fresh('s.op');
    await handleAddSocialLoginRowV0({
      filePath: fp,
      providers: [{ name: 'google' }, { name: 'github' }],
    });
    const root = getRoot(await readDoc(fp));
    const icons = findAllByRole(root, 'social-login-button-icon');
    expect(icons[0].iconFontName).toBe('chrome');
    expect(icons[1].iconFontName).toBe('github');
    expect(icons[0].iconFontFamily).toBe('lucide');
  });

  it('explicit icon overrides known-name mapping', async () => {
    const fp = await fresh('s.op');
    await handleAddSocialLoginRowV0({
      filePath: fp,
      providers: [{ name: 'google', icon: 'star' }],
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'social-login-button-icon')!.iconFontName).toBe('star');
  });

  it('unknown provider falls back to log-in icon', async () => {
    const fp = await fresh('s.op');
    await handleAddSocialLoginRowV0({
      filePath: fp,
      providers: [{ name: 'Okta' }],
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'social-login-button-icon')!.iconFontName).toBe('log-in');
  });

  it('clamps to 6 providers max', async () => {
    const fp = await fresh('s.op');
    const many = Array.from({ length: 10 }, (_, i) => ({ name: `p${i}` }));
    await handleAddSocialLoginRowV0({ filePath: fp, providers: many });
    const root = getRoot(await readDoc(fp));
    expect(findAllByRole(root, 'social-login-button')).toHaveLength(6);
  });

  it('empty providers array throws (not silent success)', async () => {
    const fp = await fresh('s.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddSocialLoginRowV0({ filePath: fp, providers: [] })).rejects.toThrow(
      /providers.*must not be empty/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });

  it('width clamps (< 200 → 200)', async () => {
    const fp = await fresh('s.op');
    await handleAddSocialLoginRowV0({
      filePath: fp,
      providers: [{ name: 'google' }],
      width: 100,
    });
    const root = getRoot(await readDoc(fp));
    expect(root.width).toBe(200);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('s.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      handleAddSocialLoginRowV0({
        filePath: fp,
        providers: [{ name: 'google' }],
        parent_id: 'nope',
      }),
    ).rejects.toThrow(/parent_id.*not found/);
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
