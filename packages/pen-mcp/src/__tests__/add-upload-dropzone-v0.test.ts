import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DESIGN_TOOL_DEFINITIONS, DESIGN_TOOL_NAMES } from '../routes/design-routes';
import { handleAddUploadDropzoneV0 } from '../tools/add-upload-dropzone-v0';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-add-upload-dropzone-v0');
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
  for (const f of ['d.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('add_upload_dropzone_v0', () => {
  it('registered; required=[] (all optional)', () => {
    expect(DESIGN_TOOL_NAMES.has('add_upload_dropzone_v0')).toBe(true);
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'add_upload_dropzone_v0');
    expect(def?.inputSchema.required).toEqual([]);
  });

  it('defaults: 480×200, upload-cloud icon, "Drop files to upload" title', async () => {
    const fp = await fresh('d.op');
    await handleAddUploadDropzoneV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    expect(root.role).toBe('upload-dropzone');
    expect(root.width).toBe(480);
    expect(root.height).toBe(200);
    const kids = root.children as Record<string, unknown>[];
    const icon = kids.find((k) => k.role === 'upload-dropzone-icon')!;
    expect(icon.iconFontName).toBe('upload-cloud');
    const title = kids.find((k) => k.role === 'upload-dropzone-title')!;
    expect(title.content).toBe('Drop files to upload');
    const subtitle = kids.find((k) => k.role === 'upload-dropzone-subtitle')!;
    expect(subtitle.content).toBe('or click to browse');
  });

  it('dashed stroke visual cue', async () => {
    const fp = await fresh('d.op');
    await handleAddUploadDropzoneV0({ filePath: fp });
    const root = getRoot(await readDoc(fp));
    const stroke = root.stroke as { strokeDashArray?: number[] };
    expect(stroke.strokeDashArray).toEqual([6, 4]);
  });

  it('custom icon + title + subtitle override defaults', async () => {
    const fp = await fresh('d.op');
    await handleAddUploadDropzoneV0({
      filePath: fp,
      icon: 'file-up',
      title: 'Drop your resume here',
      subtitle: 'PDF or DOCX, max 5 MB',
    });
    const root = getRoot(await readDoc(fp));
    const kids = root.children as Record<string, unknown>[];
    expect(kids.find((k) => k.role === 'upload-dropzone-icon')!.iconFontName).toBe('file-up');
    expect(kids.find((k) => k.role === 'upload-dropzone-title')!.content).toBe(
      'Drop your resume here',
    );
    expect(kids.find((k) => k.role === 'upload-dropzone-subtitle')!.content).toBe(
      'PDF or DOCX, max 5 MB',
    );
  });

  it('size clamps (width < 200 → 200, height < 120 → 120)', async () => {
    const fp = await fresh('d.op');
    await handleAddUploadDropzoneV0({ filePath: fp, width: 50, height: 50 });
    const root = getRoot(await readDoc(fp));
    expect(root.width).toBe(200);
    expect(root.height).toBe(120);
  });

  it('throws on bogus parent_id AND leaves file untouched', async () => {
    const fp = await fresh('d.op');
    const before = await readFile(fp, 'utf-8');
    await expect(handleAddUploadDropzoneV0({ filePath: fp, parent_id: 'nope' })).rejects.toThrow(
      /parent_id.*not found/,
    );
    expect(await readFile(fp, 'utf-8')).toBe(before);
  });
});
