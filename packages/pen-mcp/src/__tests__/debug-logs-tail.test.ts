import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, mkdir, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import { handleLogsTail, __setLogDirForTests } from '../tools/debug-logs-tail';

const TMP = join(tmpdir(), 'pen-mcp-logs-tail-tests');

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
  __setLogDirForTests(TMP);
});

afterEach(async () => {
  try {
    await rm(TMP, { recursive: true, force: true });
  } catch {}
  __setLogDirForTests(null);
});

describe('handleLogsTail', () => {
  it('returns empty when no log files exist', async () => {
    const json = await handleLogsTail({});
    const parsed = JSON.parse(json);
    expect(parsed.lines).toEqual([]);
  });

  it('reads todays log file', async () => {
    const today = new Date().toISOString().slice(0, 10);
    await writeFile(
      join(TMP, `server-${today}.log`),
      ['2026-04-06T10:00:00.000Z [INFO] alpha', '2026-04-06T10:00:01.000Z [INFO] beta'].join('\n') +
        '\n',
    );
    const json = await handleLogsTail({ tailLines: 10 });
    const parsed = JSON.parse(json);
    expect(parsed.lines).toHaveLength(2);
    expect(parsed.source).toBe('server');
  });

  it('falls back to most recent file within 7 days', async () => {
    const d = new Date();
    d.setDate(d.getDate() - 3);
    const stamp = d.toISOString().slice(0, 10);
    await writeFile(join(TMP, `server-${stamp}.log`), '2026-04-03T10:00:00.000Z [INFO] older\n');
    const json = await handleLogsTail({});
    const parsed = JSON.parse(json);
    expect(parsed.lines).toHaveLength(1);
    expect(parsed.lines[0]).toContain('older');
  });

  it('redacts sensitive lines', async () => {
    const today = new Date().toISOString().slice(0, 10);
    await writeFile(
      join(TMP, `server-${today}.log`),
      [
        '2026-04-06T10:00:00.000Z [INFO] ok',
        '2026-04-06T10:00:01.000Z ANTHROPIC_API_KEY=sk-bad',
        '2026-04-06T10:00:02.000Z [INFO] done',
      ].join('\n') + '\n',
    );
    const json = await handleLogsTail({});
    const parsed = JSON.parse(json);
    expect(parsed.lines).toHaveLength(2);
    expect(parsed.lines.some((l: string) => l.includes('ANTHROPIC_API_KEY'))).toBe(false);
  });
});
