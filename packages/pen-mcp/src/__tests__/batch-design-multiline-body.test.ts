import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { handleBatchDesign } from '../tools/batch-design';
import { invalidateCache } from '../document-manager';

/**
 * Regression for the 2026-04-21 Kimi K2.5 A/B regression: pretty-printed
 * multi-line JSON inside `I(...)` / `U(...)` / etc. must parse. `splitOperations`
 * already preserves multi-line balanced bodies; `executeLine`'s parse regex
 * had `.+` without the `s` flag, so `.` stopped at the first `\n` and the
 * whole line was mis-classified as unparseable. See
 * openpencil-docs/superpowers/notes/2026-04-21-kimi-k25-regression-rca.md.
 */
describe('batch_design — multi-line pretty-printed body (Kimi K2.5 regression)', () => {
  const TMP = join(tmpdir(), 'openpencil-batch-design-multiline');
  const EMPTY = JSON.stringify({ version: '1.0.0', children: [] });

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

  async function fresh(name: string): Promise<string> {
    const fp = join(TMP, name);
    await writeFile(fp, EMPTY, 'utf-8');
    return fp;
  }

  it('accepts an insert with pretty-printed multi-line JSON body', async () => {
    const fp = await fresh('a.op');
    const operations = [
      'root=I(null, {',
      '  "type": "frame",',
      '  "name": "Root",',
      '  "width": 375,',
      '  "height": 200,',
      '  "layout": "vertical"',
      '})',
    ].join('\n');
    const res = await handleBatchDesign({ filePath: fp, operations, postProcess: false });
    expect(res.errors).toBeUndefined();
    expect(res.results.length).toBe(1);
    const saved = JSON.parse(await readFile(fp, 'utf-8'));
    const children = (saved.children ?? saved.pages?.[0]?.children) as Record<string, unknown>[];
    expect(children.length).toBe(1);
    expect(children[0].name).toBe('Root');
    expect(children[0].layout).toBe('vertical');
  });

  it('accepts bindless insert with multi-line body (Kimi style)', async () => {
    const fp = await fresh('a.op');
    const operations =
      'I(null, {\n  "type": "frame",\n  "name": "NoBind",\n  "width": 100,\n  "height": 100\n})';
    const res = await handleBatchDesign({ filePath: fp, operations, postProcess: false });
    expect(res.errors).toBeUndefined();
    expect(res.results.length).toBe(1);
    const saved = JSON.parse(await readFile(fp, 'utf-8'));
    const children = (saved.children ?? saved.pages?.[0]?.children) as Record<string, unknown>[];
    expect(children[0].name).toBe('NoBind');
  });

  it('accepts update call with multi-line patch body (insert + update in one batch)', async () => {
    const fp = await fresh('a.op');
    // Bindings live in the per-handleBatchDesign Map — insert + update must
    // be in the same call for the `root` binding to resolve on U(). In a
    // real agent flow the two lines arrive concatenated in one payload.
    const operations = [
      'root=I(null, {"type":"frame","name":"Pre","width":100,"height":100})',
      'U(root, {',
      '  "name": "After",',
      '  "width": 200',
      '})',
    ].join('\n');
    const res = await handleBatchDesign({ filePath: fp, operations, postProcess: false });
    expect(res.errors).toBeUndefined();
    const saved = JSON.parse(await readFile(fp, 'utf-8'));
    const children = (saved.children ?? saved.pages?.[0]?.children) as Record<string, unknown>[];
    expect(children[0].name).toBe('After');
    expect(children[0].width).toBe(200);
  });

  it('still rejects genuinely malformed single-line input', async () => {
    const fp = await fresh('a.op');
    // Missing the closing `)` — should surface as an error, not silently pass
    const res = await handleBatchDesign({
      filePath: fp,
      operations: 'x=I(null, {"type":"frame","name":"Bad"',
      postProcess: false,
    });
    // splitOperations keeps unclosed lines; executeLine can't match → error collected
    expect(res.errors).toBeDefined();
    expect(res.errors!.length).toBeGreaterThan(0);
  });
});
