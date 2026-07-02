import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  getAllElementToolMetrics,
  getElementToolMetric,
  getTopElementToolCalls,
  recordElementToolCall,
  resetElementToolMetrics,
} from '../metrics/element-tool-metrics';
import { handleElementToolCall } from '../routes/element-tool-defs';
import { invalidateCache } from '../document-manager';

const TMP = join(tmpdir(), 'openpencil-element-tool-metrics');
const EMPTY = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP, name);
  await writeFile(fp, EMPTY, 'utf-8');
  return fp;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
  resetElementToolMetrics();
});
afterEach(async () => {
  for (const f of ['m.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('element-tool metrics module', () => {
  describe('direct counter API', () => {
    it('increments on first record', () => {
      recordElementToolCall('add_heading_v0', true);
      const m = getElementToolMetric('add_heading_v0');
      expect(m?.calls).toBe(1);
      expect(m?.errors).toBe(0);
    });

    it('counts successive calls', () => {
      recordElementToolCall('add_heading_v0', true);
      recordElementToolCall('add_heading_v0', true);
      recordElementToolCall('add_heading_v0', true);
      expect(getElementToolMetric('add_heading_v0')?.calls).toBe(3);
    });

    it('counts errors separately', () => {
      recordElementToolCall('add_badge_v0', true);
      recordElementToolCall('add_badge_v0', false, 'missing label');
      recordElementToolCall('add_badge_v0', false, 'bad color');
      const m = getElementToolMetric('add_badge_v0');
      expect(m?.calls).toBe(3);
      expect(m?.errors).toBe(2);
      expect(m?.lastError).toBe('bad color');
    });

    it('returns undefined for never-called tool', () => {
      expect(getElementToolMetric('add_never_called_v0')).toBeUndefined();
    });

    it('reset clears all counters', () => {
      recordElementToolCall('add_foo_v0', true);
      recordElementToolCall('add_bar_v0', false, 'oops');
      resetElementToolMetrics();
      expect(getElementToolMetric('add_foo_v0')).toBeUndefined();
      expect(getElementToolMetric('add_bar_v0')).toBeUndefined();
      expect(Object.keys(getAllElementToolMetrics()).length).toBe(0);
    });

    it('snapshot is alphabetical by name', () => {
      recordElementToolCall('add_zebra_v0', true);
      recordElementToolCall('add_apple_v0', true);
      recordElementToolCall('add_mango_v0', true);
      const keys = Object.keys(getAllElementToolMetrics());
      expect(keys).toEqual(['add_apple_v0', 'add_mango_v0', 'add_zebra_v0']);
    });

    it('top-N ranks by call count, ties broken by name', () => {
      recordElementToolCall('add_a_v0', true);
      recordElementToolCall('add_b_v0', true);
      recordElementToolCall('add_b_v0', true);
      recordElementToolCall('add_c_v0', true);
      recordElementToolCall('add_c_v0', true);
      recordElementToolCall('add_d_v0', true);

      const top = getTopElementToolCalls(3);
      expect(top[0].calls).toBe(2);
      expect(top[1].calls).toBe(2);
      // Tie breaker: names in sorted order
      expect(top[0].name).toBe('add_b_v0');
      expect(top[1].name).toBe('add_c_v0');
      expect(top[2].calls).toBe(1);
    });

    it('top-N with N=0 returns empty; N larger than set returns all', () => {
      recordElementToolCall('add_a_v0', true);
      recordElementToolCall('add_b_v0', true);
      expect(getTopElementToolCalls(0)).toEqual([]);
      expect(getTopElementToolCalls(100).length).toBe(2);
    });

    it('readers return copies — cannot mutate counter by reference', () => {
      recordElementToolCall('add_x_v0', true);
      const snap = getElementToolMetric('add_x_v0')!;
      snap.calls = 999; // local mutation
      // Internal counter must be unchanged
      expect(getElementToolMetric('add_x_v0')?.calls).toBe(1);
    });
  });

  describe('dispatcher instrumentation', () => {
    it('increments on successful element-tool call', async () => {
      const fp = await fresh('m.op');
      await handleElementToolCall('add_heading_v0', {
        filePath: fp,
        content: 'Hello',
      });
      expect(getElementToolMetric('add_heading_v0')?.calls).toBe(1);
      expect(getElementToolMetric('add_heading_v0')?.errors).toBe(0);
    });

    it('increments AND counts errors on failed call (missing required field)', async () => {
      const fp = await fresh('m.op');
      // Pass intentionally-bad parent_id to trigger the ensureParentExists throw
      await expect(
        handleElementToolCall('add_heading_v0', {
          filePath: fp,
          content: 'Hello',
          parent_id: 'nonexistent_id',
        }),
      ).rejects.toThrow(/parent_id.*not found/);
      const m = getElementToolMetric('add_heading_v0');
      expect(m?.calls).toBe(1);
      expect(m?.errors).toBe(1);
      expect(m?.lastError).toMatch(/parent_id/);
    });

    it('3 different tools in one session → 3 counters, each =1', async () => {
      const fp = await fresh('m.op');
      await handleElementToolCall('add_heading_v0', { filePath: fp, content: 'A' });
      await handleElementToolCall('add_body_text_v0', { filePath: fp, content: 'B' });
      await handleElementToolCall('add_badge_v0', { filePath: fp, label: 'NEW' });
      const snap = getAllElementToolMetrics();
      expect(Object.keys(snap).length).toBe(3);
      expect(snap.add_heading_v0.calls).toBe(1);
      expect(snap.add_body_text_v0.calls).toBe(1);
      expect(snap.add_badge_v0.calls).toBe(1);
    });

    it('unknown tool name in dispatcher still records a call (returns empty string)', async () => {
      const result = await handleElementToolCall('add_never_existed_v0', {});
      expect(result).toBe(''); // dispatcher fallthrough
      // Counter still fires — useful for detecting "AI called a tool we don't have"
      expect(getElementToolMetric('add_never_existed_v0')?.calls).toBe(1);
    });
  });
});
