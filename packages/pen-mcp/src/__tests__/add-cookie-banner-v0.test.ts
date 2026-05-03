import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddCookieBannerV0 } from '../tools/add-cookie-banner-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-cookie-banner-v0');
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
  for (const f of ['c.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_cookie_banner_v0', () => {
  it('registered; no required fields (all defaults)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_cookie_banner_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_cookie_banner_v0');
    const req = def?.inputSchema.required as string[] | undefined;
    expect(req === undefined || req.length === 0).toBe(true);
  });

  it('defaults: "We use cookies" title, generic body, "Accept all" / "Reject" buttons, no settings link', async () => {
    const fp = await fresh('c.op');
    await handleAddCookieBannerV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('cookie-banner');
    expect(findByRole(root, 'cookie-banner-title')!.content).toBe('We use cookies');
    expect(findByRole(root, 'cookie-banner-accept-label')!.content).toBe('Accept all');
    expect(findByRole(root, 'cookie-banner-decline-label')!.content).toBe('Reject');
    expect(findByRole(root, 'cookie-banner-settings')).toBeUndefined();
  });

  it('accept button uses accent fill; decline uses slate', async () => {
    const fp = await fresh('c.op');
    await handleAddCookieBannerV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    expect(fillColor(findByRole(root, 'cookie-banner-accept'))).toBe('#2563EB');
    expect(fillColor(findByRole(root, 'cookie-banner-decline'))).toBe('#F1F5F9');
  });

  it('show_settings_link=true renders the third settings link', async () => {
    const fp = await fresh('c.op');
    await handleAddCookieBannerV0({ filePath: fp, show_settings_link: true });
    const root = getRoot(await readDoc(fp));
    const link = findByRole(root, 'cookie-banner-settings')!;
    expect(link.content).toBe('Cookie settings');
    expect(fillColor(link)).toBe('#2563EB');
  });

  it('custom labels override defaults', async () => {
    const fp = await fresh('c.op');
    await handleAddCookieBannerV0({
      filePath: fp,
      title: 'Privacy choices',
      body: 'We respect your privacy.',
      accept_label: 'Allow all',
      decline_label: 'Decline non-essential',
      show_settings_link: true,
      settings_label: 'Manage preferences',
    });
    const root = getRoot(await readDoc(fp));
    expect(findByRole(root, 'cookie-banner-title')!.content).toBe('Privacy choices');
    expect(findByRole(root, 'cookie-banner-body')!.content).toBe('We respect your privacy.');
    expect(findByRole(root, 'cookie-banner-accept-label')!.content).toBe('Allow all');
    expect(findByRole(root, 'cookie-banner-decline-label')!.content).toBe('Decline non-essential');
    expect(findByRole(root, 'cookie-banner-settings')!.content).toBe('Manage preferences');
  });

  it('actions row has decline before accept (decline is left)', async () => {
    const fp = await fresh('c.op');
    await handleAddCookieBannerV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    const actions = findByRole(root, 'cookie-banner-actions')!;
    const kids = actions.children as Array<Record<string, unknown>>;
    expect(kids[0].role).toBe('cookie-banner-decline');
    expect(kids[1].role).toBe('cookie-banner-accept');
  });

  it('width clamps (< 320 → 320)', async () => {
    const fp = await fresh('c.op');
    await handleAddCookieBannerV0({ filePath: fp, width: 100 });
    const root = getRoot(await readDoc(fp));
    expect(root.width).toBe(320);
  });

  it('emits a shadow effect on the banner card', async () => {
    const fp = await fresh('c.op');
    await handleAddCookieBannerV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    const effects = root.effects as Array<{ type?: string }> | undefined;
    expect(effects?.[0]?.type).toBe('shadow');
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('c.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddCookieBannerV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
